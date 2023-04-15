use std::collections::HashMap;
use std::fs::{copy, create_dir_all};
use std::mem::{replace};
use std::path::{Path, PathBuf};
use clap::{arg, ArgAction, ArgMatches, Command, value_parser};
use clap::builder::ArgPredicate;
use multimap::MultiMap;
use path_clean::PathClean;
use fontpm_api::{error, FpmHost, info, ok, Source, trace, warning};
use fontpm_api::font::{DefinedFontInstallSpec, DefinedFontStyle, DefinedFontVariantSpec, DefinedFontWeight, FontDescription, FontInstallSpec};
use fontpm_api::util::{nice_list, plural_s, plural_s_opposite};
use crate::commands::{CommandAndRunner, Error};
use crate::config::FpmConfig;
use crate::generate::Generate;
use crate::host_impl::FpmHostImpl;
use crate::runner;
use crate::sources::{create_sources, FontSpec};

pub const NAME: &str = "install";

#[derive(clap_derive::ValueEnum, Clone)]
#[value(rename_all = "kebab-case")]
enum OutputFormat {
    // {font id}-{variant weight}
    Flat,
    FlatDirectory,
}
impl OutputFormat {
    fn get_path(&self, base_dir: impl AsRef<Path>, font_spec: &DefinedFontInstallSpec, variant_spec: &DefinedFontVariantSpec, source_path: impl AsRef<Path>) -> PathBuf {
        let base_dir = base_dir.as_ref();
        let source_path = source_path.as_ref();
        let ext = source_path.extension().map(|v| String::from(".") + v.to_str().unwrap()).unwrap_or("".to_string());
        let file_name = format!("{}{}{}", font_spec.id, {
            if variant_spec == &DefinedFontVariantSpec::REGULAR {
                "-regular".to_string()
            } else {
                let style = match variant_spec.style {
                    DefinedFontStyle::Regular => "",
                    DefinedFontStyle::Italic => "-italic",
                };
                match variant_spec.weight {
                    DefinedFontWeight::REGULAR => style.to_string(),
                    DefinedFontWeight::Fixed(weight) => "-".to_string() + weight.to_string().as_str() + style,
                    DefinedFontWeight::Variable => "-wght".to_string() + style
                }
            }
        }, ext);
        match self {
            Self::Flat => {
                base_dir.join(file_name)
            }
            Self::FlatDirectory => {
                base_dir.join(&font_spec.id).join(file_name)
            }
        }
    }
    fn get_misc_path(&self, base_dir: impl AsRef<Path>, font_desc: &FontDescription, name: impl AsRef<Path>) -> PathBuf {
        let base_dir = base_dir.as_ref();
        let name = name.as_ref();
        match self {
            Self::Flat => base_dir.join(name),
            Self::FlatDirectory => base_dir.join(&font_desc.id).join(name)
        }
    }
}

async fn _runner(args: &ArgMatches) -> Result<Option<String>, Error> {
    let config = FpmConfig::load()?;

    let fontspecs = args.get_many::<String>("fontspec");
    if fontspecs.is_none() {
        return Err(Error::Custom("No fonts specified.".into()));
    }
    let fontspecs = fontspecs.unwrap();

    if fontspecs.len() < 1 { // Logically shouldn't happen but just in case
        return Err(Error::Custom("At least one fontspec must be provided".into()));
    }

    let fontspecs: Vec<(String, fontpm_api::Result<FontSpec>)> = fontspecs.into_iter()
        .map(|v| (v.clone(), FontSpec::parse(v)))
        .collect();

    let fontspecs = {
        let mut vec: Vec<FontSpec> = Vec::new();
        for item in fontspecs.iter() {
            let (original, result) = item;
            match result {
                Err(error) => {
                    warning!("Error whilst parsing fontspec \"{}\": {}", original, error);
                    continue;
                },
                Ok(fontspec) => {
                    vec.push(fontspec.clone());
                }
            }
        }
        (vec, fontspecs.len())
    };

    if fontspecs.0.len() < 1 {
        return Err(Error::Custom(format!(
            "{}. Perhaps you made a typo?",
            if fontspecs.1 == 1 {
                "The fontspec was invalid"
            } else {
                "All fontspecs were invalid"
            }
        )))
    }

    let fontspecs = fontspecs.0;
    // TODO: Make sure no two fontspecs conflict

    let required_sources = if fontspecs.iter().any(|v| v.source.is_none()) {
        None
    } else {
        Some(
            fontspecs.iter()
                .map(|v| v.source.as_ref().unwrap())
                .collect()
        )
    };

    let host = FpmHostImpl::create(Some(config.font_install_dir()))?;
    let sources = create_sources(Some(&host), required_sources.clone())?;

    if sources.is_empty() {
        if let Some(only) = required_sources {
            let s = plural_s(only.len());
            let s_opposite = plural_s_opposite(only.len());
            error!("No source{} with the ID{} {} exist{} (perhaps you have the source{} disabled?)",
                s,
                s,
                nice_list(only.clone(), "and"),
                s_opposite,
                s
            );
        } else {
            error!("No sources are enabled. Please enable sources in your configuration file.");
        }
    }

    let mut fontspec_by_source = MultiMap::new();
    for fontspec in &fontspecs {
        fontspec_by_source.insert(fontspec.source.clone(), fontspec.clone());
    }

    let mut resolved = HashMap::new();
    let mut errors = false;
    for source in fontspec_by_source.keys() {
        // NOTE(tecc): `None` means "any source"
        let fonts_to_download = fontspec_by_source.get_vec(source).unwrap();

        let target_sources: Vec<_> = if let Some(source_name) = source {
            sources.iter()
                .filter(|v| v.name() == source_name)
                .collect()
        } else {
            sources.iter().collect()
        };

        let mut resolved_from_source = HashMap::new();
        for source in &target_sources {
            trace!("Running on source {}", source.id());
            let resolved = fonts_to_download.iter()
                .map(|fontspec| async move {
                    (fontspec.clone(), source.description(), source.resolve_font(&FontInstallSpec::new_all_styles(&fontspec.font_id)).await)
                });
            let resolved = futures::future::join_all(resolved).await;
            for resolved in resolved {
                if let Some((_, _, Ok(_))) = resolved_from_source.get(&resolved.0.font_id) {
                    continue
                }
                resolved_from_source.insert(resolved.0.font_id.clone(), resolved);
            }
        }

        for entry in resolved_from_source {
            match entry.1.2 {
                Ok(v) => {
                    if !resolved.contains_key(&entry.0) {
                        resolved.insert(entry.0, (entry.1.0, entry.1.1, v.clone()));
                    }
                },
                Err(e) => {
                    let source_name = if target_sources.len() > 1 {
                        "any of the sources"
                    } else {
                        target_sources.first().unwrap().name()
                    };
                    error!("Could not resolve font {} from {}: {}", entry.0, source_name, e);
                    errors = true;
                }
            }
        }
        // println!("{:?}", all_resolved);
    }
    if errors {
        return Err(Error::Custom(format!("Some fonts failed to resolve.")));
    }

    let (directory, output_format, generate_css) = match args.get_one::<PathBuf>("directory") {
        None => (host.font_install_dir(), OutputFormat::FlatDirectory, false),
        Some(dir) => (dir.clone(), args.get_one("format").unwrap_or(&OutputFormat::FlatDirectory).clone(), args.get_flag("generate-css"))
    };
    let directory = {
        let mut dir = directory.clean();
        if dir.is_relative() {
            let mut pwd = std::env::current_dir()?;
            pwd.push(dir);
            dir = pwd;
        }
        dir
    };
    create_dir_all(&directory)?;
    let sources: HashMap<_, _> = sources.into_iter().map(|v| (v.id().to_string(), v)).collect();

    for resolved in &resolved {
        let (_, (_font_spec, source_desc, (install_spec, font_desc))) = resolved;
        let source = sources.get(&source_desc.id).expect("logic error");
        info!("Installing {} from {}", font_desc.name, source_desc.name);
        match source.download_font(&install_spec, &host.cache_dir_for(source.id())).await {
            Ok(mut paths) => {
                for (spec, path) in &mut paths {
                    let target_path = output_format.get_path(&directory, &install_spec, &spec, &path) ;
                    trace!("Copying cache file {} to target path {}", path.display(), target_path.display());
                    if let Some(parent) = target_path.parent() {
                        create_dir_all(parent)?;
                    }
                    copy(&path, &target_path)?;
                    if generate_css {
                        let _ = replace(path, target_path);
                    }
                }
                if generate_css {
                    let target = output_format.get_misc_path(&directory, &font_desc, format!("{}.css", font_desc.id));
                    let generate = Generate::from_font(&target, &font_desc, paths);
                    let generated = generate.generate_css()?;
                    tokio::fs::write(&target, generated).await?;
                }
            },
            Err(e) => {
                return Err(Error::Custom(format!("Could not download font {} from {}: {}", font_desc.name, source_desc.name, e,)))
            }
        }
        // info!("Installed font!");
    }

    // dbg!(resolved);

    let fonts = match resolved.len() {
        1 => format!("font {}", resolved.values().last().unwrap().2.1.name),
        len => format!("{} fonts", len)
    };
    let sources = match sources.len() {
        1 => sources.values().last().unwrap().name().to_string(),
        len => format!("{} sources", len)
    };
    ok!("Successfully installed {} from {}!", fonts, sources);

    Ok(None)
}

runner! { args => _runner(args).await }

pub fn command() -> CommandAndRunner {
    return CommandAndRunner {
        description: Command::new(NAME)
            .about("Install a font")
            .args(vec![
                arg!(<fontspec> "Specify the fonts to install")
                    .long_help(
"Specify the fonts to install.
You can either specify it as simply a font ID (e.g. \"noto-sans\"),
or as <source ID>:<font ID> (e.g. \"google-fonts:noto-sans\")."
                    )
                    .action(ArgAction::Append)
                    .required(true),
                arg!(-d --directory <path> "The directory to install the fonts to.")
                    .value_parser(value_parser!(PathBuf))
                    .required(false),
                arg!(-f --format <format> "The format to install the fonts in. Will be ignored without -d.")
                    .value_parser(value_parser!(OutputFormat))
                    .required(false)
                    .default_value_if("directory", ArgPredicate::IsPresent, "flat-directory"),
                arg!(--"generate-css" "Generate @font-face rules for CSS. Will be ignored without -d.")
                    .alias("css")
                    .action(ArgAction::SetTrue)
            ])
        ,
        runner: Box::new(runner)
    };
}