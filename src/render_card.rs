use std::{collections::HashMap, sync::Arc};

use resvg::usvg::{ImageKind, ImageRendering, TreeParsing, TreeTextToPath};

#[derive(serde::Serialize)]
pub struct Context {
    pub level: u64,
    pub rank: i64,
    pub name: String,
    pub discriminator: String,
    pub percentage: u64,
    pub current: u64,
    pub needed: u64,
    pub toy: Option<String>,
    pub avatar: String,
}

// the render can take a while, so this is a relatively thin function
pub async fn render(state: SvgState, context: Context) -> Result<Vec<u8>, RenderingError> {
    let context = tera::Context::from_serialize(context)?;
    tokio::task::spawn_blocking(move || do_render(&state, &context)).await?
}

fn do_render(state: &SvgState, context: &tera::Context) -> Result<Vec<u8>, RenderingError> {
    // this actually just does the templating, which has pretty much all been set up already.
    let svg = state.tera.render("svg", context)?;
    // the data resolver is used to support user PFPs
    let resolve_data = Box::new(
        |mime: &str, data: std::sync::Arc<Vec<u8>>, _opt: &resvg::usvg::Options| match mime {
            "image/png" => Some(ImageKind::PNG(data)),
            "image/jpg" | "image/jpeg" => Some(ImageKind::JPEG(data)),
            _ => None,
        },
    );
    // All string images come from a static hashmap initialized with [`SvgState`]
    let resolve_string_state = state.clone();
    let resolve_string = Box::new(move |href: &str, _: &resvg::usvg::Options| {
        Some(ImageKind::PNG(
            resolve_string_state.images.get(href)?.clone(),
        ))
    });
    let opt = resvg::usvg::Options {
        image_href_resolver: resvg::usvg::ImageHrefResolver {
            resolve_data,
            resolve_string,
        },
        // This enables crisp-edge rendering for our resources, which improves the quality of CEa_TiDE's pixelart.
        image_rendering: ImageRendering::OptimizeSpeed,
        ..Default::default()
    };
    let mut tree = resvg::usvg::Tree::from_str(&svg, &opt)?;
    tree.convert_text(&state.fonts);
    let pixmap_size = tree.size.to_screen_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height())
        .ok_or(RenderingError::PixmapCreation)?;
    resvg::render(
        &tree,
        resvg::FitTo::Original,
        resvg::tiny_skia::Transform::default(),
        pixmap.as_mut(),
    );
    Ok(pixmap.encode_png()?)
}

#[derive(Clone)]
pub struct SvgState {
    fonts: Arc<resvg::usvg::fontdb::Database>,
    tera: Arc<tera::Tera>,
    images: Arc<HashMap<String, Arc<Vec<u8>>>>,
}

impl SvgState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for SvgState {
    fn default() -> Self {
        let mut fonts = resvg::usvg::fontdb::Database::new();
        fonts.load_font_data(include_bytes!("resources/Mojang.ttf").to_vec());
        let mut tera = tera::Tera::default();
        tera.autoescape_on(vec!["svg", "html", "xml", "htm"]);
        tera.add_raw_template("svg", include_str!("resources/card.svg"))
            .expect("Failed to build card.svg template!");
        let images = HashMap::from([
            (
                "parrot.png".to_string(),
                Arc::new(include_bytes!("resources/icons/CEa_TIde/parrot.png").to_vec()),
            ),
            (
                "fox.png".to_string(),
                Arc::new(include_bytes!("resources/icons/CEa_TIde/fox.png").to_vec()),
            ),
            (
                "grassblock.png".to_string(),
                Arc::new(include_bytes!("resources/icons/CEa_TIde/grassblock.png").to_vec()),
            ),
            (
                "pickaxe.png".to_string(),
                Arc::new(include_bytes!("resources/icons/CEa_TIde/pickaxe.png").to_vec()),
            ),
            (
                "steveheart.png".to_string(),
                Arc::new(include_bytes!("resources/icons/CEa_TIde/steveheart.png").to_vec()),
            ),
            (
                "tree.png".to_string(),
                Arc::new(include_bytes!("resources/icons/CEa_TIde/tree.png").to_vec()),
            ),
            (
                "airplane.png".to_string(),
                Arc::new(include_bytes!("resources/icons/valkyrie_pilot/airplane.png").to_vec()),
            ),
        ]);
        Self {
            fonts: Arc::new(fonts),
            tera: Arc::new(tera),
            images: Arc::new(images),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RenderingError {
    #[error("Tera error: {0}")]
    Template(#[from] tera::Error),
    #[error("Tokio JoinError: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("uSVG error: {0}")]
    Usvg(#[from] resvg::usvg::Error),
    #[error("Pixmap error: {0}")]
    Pixmap(#[from] png::EncodingError),
    #[error("Pixmap Creation error!")]
    PixmapCreation,
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    #[test]
    fn test_renderer() -> Result<(), RenderingError> {
        let state = SvgState::new();
        let xp = rand::thread_rng().gen_range(0..=10_000_000);
        let data = mee6::LevelInfo::new(xp);
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation
        )]
        let context = Context {
            level: data.level(),
            rank: rand::thread_rng().gen_range(0..=1_000_000),
            name: "Testy McTestington<span>".to_string(),
            discriminator: "0000".to_string(),
            percentage: (data.percentage() * 100.0).round() as u64,
            current: xp,
            needed: mee6::xp_needed_for_level(data.level() + 1),
            toy: Some("parrot.png".to_string()),
            avatar: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAQAAAAEABAMAAACuXLVVAAAAIGNIUk0AAHomAACAhAAA+gAAAIDoAAB1MAAA6mAAADqYAAAXcJy6UTwAAAAYUExURXG0zgAAAFdXV6ampoaGhr6zpHxfQ2VPOt35dJcAAAABYktHRAH/Ai3eAAAAB3RJTUUH5wMDFSE5W/eo1AAAAQtJREFUeNrt1NENgjAUQFFXYAVWYAVXcAVXYH0hoQlpSqGY2Dae82WE9971x8cDAAAAAAAAAAAAAAAAAADgR4aNAAEC/jNgPTwuBAgQ8J8B69FpI0CAgL4DhozczLgjQICAPgPCkSkjtXg/I0CAgD4Dzg4PJ8YEAQIE9BEQLyg5cEWYFyBAQHsBVxcPN8U7BAgQ0FbAlcNhcLohjkn+egECBFQPKPE8cXpQgAABzQXkwsIfUElwblaAAAF9BeyP3Z396rgAAQJ+EvCqTIAAAfUD3pUJECCgvYB5kfp89N28yR3J7RQgQED9gPjhfmG8/Oh56r1UYOpdAQIEtBFwtLBUyY7wrgABAqoHfABW2cbX3ElRgQAAACV0RVh0ZGF0ZTpjcmVhdGUAMjAyMy0wMy0wM1QyMTozMzo1NiswMDowMNpnAp0AAAAldEVYdGRhdGU6bW9kaWZ5ADIwMjMtMDMtMDNUMjE6MzM6NTYrMDA6MDCrOrohAAAAKHRFWHRkYXRlOnRpbWVzdGFtcAAyMDIzLTAzLTAzVDIxOjMzOjU3KzAwOjAwWliQSgAAAABJRU5ErkJggg==".to_string(),
        };
        let output = do_render(&state, &tera::Context::from_serialize(context)?)?;
        std::fs::write("renderer_test.png", output).unwrap();
        Ok(())
    }
}
