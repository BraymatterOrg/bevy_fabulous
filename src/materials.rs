use std::marker::PhantomData;

use bevy::{prelude::*, utils::HashMap};

/// Handles automatically swapping out materials with a specific name from a GLTF / Scene with a specific material. 
/// If you're using the StandardMaterial you can probably fiddle with the material in blender to get what you want, 
/// but if you're using a custom Material, or some particularly complicated StandardMaterials this gives provides 
/// for a way to swap materials out as desired
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
    
    /// Watch asset_loaded events for GLTF's to be loaded, if they contained named materials this will
    /// check whether they should be overriden
    /// Note: When loading a Scene Asset directly, it seems as though the GLTF is discarded after it is loaded.
    /// This system needs the GLTF asset as that's what contains the HashMap<MaterialName, Handle<StandardMaterial>> 
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
                        debug!("Found Named Material: {}", named);
                        if mat_registry.contains_override(named.to_string()) {
                            if let Err(e) =
                                mat_registry.register_swap_mat(named.to_string(), &mat.clone())
                            {
                                error!("Error registering new swap material: \n {}", e);
                            } else {
                                debug!("Registering Swap Mat!")
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Used to track which material handles should be swapped for a 'main-material'
/// Multiple materials can be swapped for the same main material
#[derive(Resource)]
pub struct NamedMaterialIndex<T: Material, G: Material> {
    /// Contains a map of the material name, to a tuple of the final mat handle, and any materials that should be replaced by it
    pub materials_to_swap: HashMap<String, (Handle<T>, Vec<Handle<G>>)>,
}

impl<T: Material, G: Material> NamedMaterialIndex<T, G> {
    /// Creates a new, empty material index
    pub fn new() -> Self {
        Self {
            materials_to_swap: HashMap::new(),
        }
    }
    
    /// Register a new main material, materials loaded from GLTF's (Really anywhere) will be swapped out for the main material
    pub fn register_main_mat(&mut self, name: impl Into<String>, mat: Handle<T>) {
        self.materials_to_swap.insert(name.into(), (mat, vec![]));
    }

    /// Register a swap material. The material handle will be removed from the entity, and the main material handle will be added
    pub fn register_swap_mat(
        &mut self,
        name: impl Into<String>,
        mat: &Handle<G>,
    ) -> Result<(), &str> {
        let n = name.into();
        let Some((_master, swaps)) = self.materials_to_swap.get_mut(&n) else {
            return Err("No Master Material specified for name: {}");
        };
        
        //Clone weak so just having this material in the array won't keep it alive / held if it's not used anywhere else
        swaps.push(mat.clone_weak());

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
    
    /// Returns whether a material should be swapped / overriden with a main material
    pub fn contains_override(&self, name: String) -> bool {
        self.materials_to_swap.contains_key(&name)
    }
}