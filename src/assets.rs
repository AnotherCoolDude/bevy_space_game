use std::fs;

use bevy::prelude::*;
use bevy::reflect::erased_serde::private::serde::Deserialize;
use bevy::utils::HashMap;
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use bevy_loading::prelude::AssetsLoading;
use serde::Serialize;

use crate::world_object::WorldObject;
use ron::de::from_str;

pub struct GameAssetsPlugin;

#[derive(Inspectable, Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub enum Orientation {
    Up,
    Down,
    Left,
    Right,
}

impl Default for Orientation {
    fn default() -> Self {
        Orientation::Down
    }
}

//FIXME Graphics and Graphic are too confusing of names
#[derive(
    Inspectable, Component, Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone, Copy,
)]
pub enum Graphic {
    Player(Orientation),
    WorldObject(WorldObject),
}

//All sheets in the assets
#[derive(Clone, Copy, Debug, Reflect, Deserialize, PartialEq, Eq, Hash)]
pub enum SpriteSheet {
    Character,
    StarterGraphics,
}

//Entry on the ron sheet description
#[derive(Clone, Copy, Debug, Reflect, Deserialize)]
pub struct SpriteDesc {
    pub sheet: SpriteSheet,
    pub min: Vec2,
    pub max: Vec2,
    #[serde(default)]
    flip_x: bool,
    #[serde(default)]
    flip_y: bool,
}

//Resource holding all handles and indices
//XXX Make sure the performance isn't trash...
pub struct Graphics {
    pub handle_map: HashMap<SpriteSheet, Handle<TextureAtlas>>,
    pub graphics_map: HashMap<Graphic, (SpriteDesc, usize)>,
}

//Format to be loaded from ron
#[derive(Deserialize)]
pub struct GraphicsDesc {
    sheet_filename_map: HashMap<SpriteSheet, String>,
    graphics_map: HashMap<Graphic, SpriteDesc>,
}

impl Plugin for GameAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system_to_stage(
            StartupStage::PreStartup,
            Self::load_graphics.label("graphics"),
        )
        .register_inspectable::<Graphic>()
        .add_system(update_sprite);
        //.add_system(Self::set_img_sampler_filter);
    }
}
//XXX Does not work if changed to graphic on another sheet
pub fn update_sprite(
    mut update_query: Query<(&mut TextureAtlasSprite, &Graphic), Changed<Graphic>>,
    graphics: Res<Graphics>,
) {
    for (mut sprite, graphic) in update_query.iter_mut() {
        if let Some((_, index)) = graphics.graphics_map.get(graphic) {
            sprite.index = *index;
        } else {
            error!(
                "Failed to load sprite for {:?}, missing in graphics_desc.ron?",
                graphic
            );
        }
    }
}

pub fn spawn_sprite(commands: &mut Commands, res: &Graphics, to_spawn: Graphic) -> Entity {
    if let Some((desc, index)) = res.graphics_map.get(&to_spawn) {
        let mut sprite = TextureAtlasSprite::new(*index);
        sprite.flip_x = desc.flip_x;
        sprite.flip_y = desc.flip_y;
        let atlas = &res.handle_map[&desc.sheet];
        commands
            .spawn_bundle(SpriteSheetBundle {
                sprite: sprite,
                texture_atlas: atlas.clone(),
                transform: Transform {
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(to_spawn)
            .id()
    } else {
        error!(
            "Failed to load sprite for {:?}, missing in graphics_desc.ron?",
            to_spawn
        );
        commands
            .spawn()
            .insert(Transform::default())
            .insert(GlobalTransform::default())
            .id()
    }
}

impl GameAssetsPlugin {
    fn load_graphics(
        mut commands: Commands,
        assets: Res<AssetServer>,
        mut texture_assets: ResMut<Assets<TextureAtlas>>,
        mut loading: ResMut<AssetsLoading>,
    ) {
        let sprite_desc = fs::read_to_string("assets/graphics_desc.ron").unwrap();

        let sprite_desc: GraphicsDesc = match from_str(&sprite_desc) {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config: {}", e);
                std::process::exit(1);
            }
        };

        //Load all images and create atlases
        let mut atlas_map = HashMap::default();
        for (sheet, file_name) in sprite_desc.sheet_filename_map.iter() {
            let image_handle = assets.load(file_name);
            loading.add(&image_handle);
            //FIXME image size should either come from loaded image or from desc ron
            let atlas = TextureAtlas::new_empty(image_handle, Vec2::splat(256.0));
            atlas_map.insert(*sheet, atlas);
        }

        //Add all sprites to their atlases and save the index
        let mut graphics_map = HashMap::default();
        for (item, desc) in sprite_desc.graphics_map.iter() {
            println!("Found graphic {:?}", item);
            let atlas = atlas_map.get_mut(&desc.sheet).unwrap();
            let index = atlas.add_texture(bevy::sprite::Rect {
                min: desc.min,
                max: desc.max,
            });
            graphics_map.insert(*item, (*desc, index));
        }

        //Save the handles
        let mut handle_map = HashMap::default();
        for (sheet, atlas) in atlas_map {
            let atlas_handle = texture_assets.add(atlas);
            handle_map.insert(sheet, atlas_handle);
        }

        commands.insert_resource(Graphics {
            handle_map: handle_map,
            graphics_map: graphics_map,
        });
    }
}
