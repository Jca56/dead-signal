//! What's loaded as the game starts, beyond the title's scene: what every
//! map is built from (scenery, containers, the ways out), and every kind
//! of thing (its mesh, and its picture for the inventory).

use lntrn_app::lntrn_render::{Gpu, Images};
use lntrn_core::{log_error, log_info};
use lntrn_math::Vec3;

use super::DeadSignal;
use crate::assets;
use crate::exits::{self, Way};
use crate::loot::tables::Source;
use crate::map::scatter::Scenery;
use crate::render::Renderer;
use crate::{containers, icons, loot};

impl DeadSignal {
    /// Everything a map is made of, loaded once: what the scenery,
    /// containers and ways out look like and what's solid of them (the
    /// kit every map is built from); and every kind of thing (its mesh, and
    /// its picture for the inventory).
    pub(super) fn load_things(&mut self, renderer: &mut Renderer, gpu: &Gpu, images: &mut Images) {
        match assets::load(renderer, "scenery") {
            Ok(props) => {
                for what in Scenery::ALL {
                    let name = what.name();
                    let find = |n: &str| props.iter().find(|p| p.name == n);
                    if let Some(hull) = find(&format!("{name}_Hull")) {
                        self.kit.scenery.insert(what, hull.triangles.clone());
                    }
                    let Some(p) = find(&name) else {
                        log_error!("scenery: no {name}");
                        continue;
                    };
                    if let Some(mesh) = p.mesh {
                        // The ball it lies in, about its origin (scaled up
                        // with it).
                        let (mut lo, mut hi) = (Vec3::splat(f64::INFINITY), Vec3::splat(f64::NEG_INFINITY));
                        for q in p.triangles.iter().flatten() {
                            lo = lo.min(*q);
                            hi = hi.max(*q);
                        }
                        self.scenery.insert(what, (mesh, (lo + hi) * 0.5, (hi - lo).length() * 0.5));
                    }
                }
            }
            Err(e) => log_error!("scenery: {e}"),
        }
        match assets::load(renderer, "containers") {
            Ok(props) => {
                for source in [Source::Crate, Source::Locker, Source::Car, Source::Cage] {
                    let name = containers::model_name(source);
                    let open = format!("{name}_Open");
                    let find = |n: &str| props.iter().find(|p| p.name == n);
                    if let Some(hull) = find(&format!("{name}_Hull")) {
                        self.kit.containers.insert(source, hull.triangles.clone());
                    }
                    if let (Some(shut), Some(open)) = (find(name).and_then(|p| p.mesh), find(&open).and_then(|o| o.mesh)) {
                        self.container_meshes.insert(source, (shut, open));
                    }
                }
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
                self.kit.exits = shapes;
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
