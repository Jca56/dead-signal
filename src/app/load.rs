//! What's loaded as the game starts, beyond the scenes themselves: the
//! containers and the ways out (set down and made solid), and every kind
//! of thing (its mesh, and its picture for the inventory).

use std::collections::HashMap;

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_core::{log_error, log_info};

use super::DeadSignal;
use crate::assets;
use crate::exits::{self, Way};
use crate::loot::tables::Source;
use crate::render::Renderer;
use crate::world::Solid;
use crate::{containers, icons, loot};

impl DeadSignal {
    pub(super) fn load_things(&mut self, renderer: &mut Renderer, gpu: &Gpu, images: &mut Images) {
        match assets::load(renderer, "containers") {
            Ok(props) => {
                let mut shapes = HashMap::new();
                let mut meshes = HashMap::new();
                for source in [Source::Crate, Source::Locker, Source::Car, Source::Cage] {
                    let name = containers::model_name(source);
                    let open = format!("{name}_Open");
                    let find = |n: &str| props.iter().find(|p| p.name == n);
                    if let Some(hull) = find(&format!("{name}_Hull")) {
                        shapes.insert(source, hull.triangles.clone());
                    }
                    if let (Some(shut), Some(open)) = (find(name).and_then(|p| p.mesh), find(&open).and_then(|o| o.mesh)) {
                        meshes.insert(source, (shut, open));
                    }
                }
                let placed = containers::set_down(&mut self.game.world.resource_mut::<Solid>().0, &shapes);
                containers::spawn(&mut self.game.world, placed, &meshes);
            }
            Err(e) => log_error!("containers: {e}"),
        }
        match assets::load(renderer, "exits") {
            Ok(props) => {
                let find = |n: &str| props.iter().find(|p| p.name == n);
                let tris = |n: &str| find(n).map(|p| p.triangles.clone()).unwrap_or_default();
                let mesh = |n: &str| find(n).and_then(|p| p.mesh);
                let mut shapes = exits::Shapes { meshes: Default::default(), hulls: Default::default(), barricade: tris("EXIT_Gate_Barricade_Hull") };
                for way in [Way::Radio, Way::Road, Way::Truck] {
                    let (normal, alt) = way.models();
                    shapes.hulls.insert(way, tris(&format!("{normal}_Hull")));
                    if let (Some(a), Some(b)) = (mesh(normal), mesh(alt)) {
                        shapes.meshes.insert(way, (a, b));
                    }
                }
                let mut placed = exits::set_down(&mut self.game.world.resource_mut::<Solid>().0, &shapes);
                let ground = self.game.ground();
                placed.ring_zones(|centre, radius| renderer.add_mesh(&exits::ring(ground, centre, radius)));
                self.game.world.insert_resource(placed);
            }
            Err(e) => log_error!("exits: {e}"),
        }
        match assets::load(renderer, "items") {
            Ok(props) => {
                let started = std::time::Instant::now();
                let mut meshes = crate::items::Meshes::default();
                for kind in loot::ALL {
                    let def = kind.def();
                    let Some(p) = props.iter().find(|p| p.name == def.model) else {
                        log_error!("items: no {}", def.model);
                        continue;
                    };
                    if let Some(mesh) = p.mesh {
                        meshes.0.insert(kind, mesh);
                    }
                    let picture = icons::draw(&p.vertices, def.size);
                    let turned = icons::turned(&picture);
                    self.icons.0.insert(kind, (images.add(gpu, &picture), images.add(gpu, &turned)));
                }
                self.game.world.insert_resource(meshes);
                log_info!("icons: drawn in {:.0} ms", started.elapsed().as_secs_f64() * 1000.0);
            }
            Err(e) => log_error!("items: {e}"),
        }
    }
}
