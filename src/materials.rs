use std::marker::PhantomData;

use bevy::{prelude::*, utils::HashMap};

#[derive(Default)]
pub struct FabulousMaterialsPlugin<T: Material> {
    p: PhantomData<T>,
}

impl<T: Material> Plugin for FabulousMaterialsPlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(NamedMaterialIndex::<T, StandardMaterial>::new());
        app.add_systems(PreUpdate, (Self::replace_materials, Self::asset_watcher));
    }
}

impl<T: Material> FabulousMaterialsPlugin<T> {
    /// Any time a material of the specified type is added, check it against the index of forbidden materials. If it is present
    /// make the swap
    fn replace_materials(
        mut cmds: Commands,
        added_mats: Query<(Entity, &Handle<StandardMaterial>), Added<Handle<StandardMaterial>>>,
        index: Res<NamedMaterialIndex<T, StandardMaterial>>,
    ) {
        for (mat_ent, handle) in added_mats.iter() {
            if let Some(mat_to_swap) = index.get_swap_mat(handle) {
                cmds.entity(mat_ent)
                    .remove::<Handle<StandardMaterial>>()
                    .insert(mat_to_swap);
            }
        }
    }

    fn asset_watcher(
        mut asset_events: EventReader<AssetEvent<Gltf>>,
        mut mat_registry: ResMut<NamedMaterialIndex<T, StandardMaterial>>,
        gltfs: Res<Assets<Gltf>>,
    ) {
        for event in asset_events.read() {
            match event {
                AssetEvent::LoadedWithDependencies { id } => {
                    let Some(gltf) = gltfs.get(*id) else {
                        error!("Received Asset Loaded Event for GLTF but no gltf found in assets");
                        continue;
                    };

                    for (named, mat) in gltf.named_materials.iter() {
                        info!("Found Named Material: {}", named);
                        if mat_registry.contains_override(named.to_string()) {
                            if let Err(e) =
                                mat_registry.register_swap_mat(named.to_string(), mat.clone())
                            {
                                error!("Error registering new swap material: \n {}", e);
                            } else {
                                info!("Registering Swap Mat!")
                            }
                        }
                    }
                    info!("Load Asset Event!")
                }
                _ => {}
            }
        }
    }
}

#[derive(Resource)]
pub struct NamedMaterialIndex<T: Material, G: Material> {
    /// Contains a map of the material name, to a tuple of the final mat handle, and any materials that should be replaced by it
    pub materials_to_swap: HashMap<String, (Handle<T>, Vec<Handle<G>>)>,
}

impl<T: Material, G: Material> NamedMaterialIndex<T, G> {
    pub fn new() -> Self {
        Self {
            materials_to_swap: HashMap::new(),
        }
    }
    pub fn register_main_mat(&mut self, name: impl Into<String>, mat: Handle<T>) {
        self.materials_to_swap.insert(name.into(), (mat, vec![]));
    }

    pub fn register_swap_mat(
        &mut self,
        name: impl Into<String>,
        mat: Handle<G>,
    ) -> Result<(), &str> {
        let n = name.into();
        let Some((_master, swaps)) = self.materials_to_swap.get_mut(&n) else {
            return Err("No Master Material specified for name: {}");
        };

        swaps.push(mat);

        Ok(())
    }

    /// Takes a potential swap material and checks if it is already in the registry
    pub fn get_swap_mat(&self, mat: &Handle<G>) -> Option<Handle<T>> {
        for (_name, (main_mat, swaps)) in self.materials_to_swap.iter() {
            if swaps.contains(mat) {
                return Some(main_mat.clone());
            }
        }

        None
    }

    pub fn contains_override(&self, name: String) -> bool {
        self.materials_to_swap.contains_key(&name)
    }
}
