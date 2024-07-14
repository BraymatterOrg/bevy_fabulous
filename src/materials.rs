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
        app.add_event::<SwapEvent>();
        app.insert_resource(FabMaterialOverrides::<T, StandardMaterial>::new());
        app.add_systems(PostUpdate, (Self::replace_materials, Self::asset_watcher));
    }
}

impl<T: Material> FabulousMaterialsPlugin<T> {
    /// Any time a material of the specified type is added, check it against the index of forbidden materials. If it is present
    /// make the swap
    fn replace_materials(
        mut cmds: Commands,
        added_mats: Query<(Entity, &Handle<StandardMaterial>), Added<Handle<StandardMaterial>>>,
        index: Res<FabMaterialOverrides<T, StandardMaterial>>,
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
        mut mat_registry: ResMut<FabMaterialOverrides<T, StandardMaterial>>,
        mut events: EventWriter<SwapEvent>,
        gltfs: Res<Assets<Gltf>>,
    ) {
        for event in asset_events.read() {
            match event {
                AssetEvent::LoadedWithDependencies { id } => {
                    let Some(gltf) = gltfs.get(*id) else {
                        error!("Received Asset Loaded Event for GLTF but no gltf found in assets");
                        continue;
                    };

                    //For every named material in the gltf
                    for (named, mat) in gltf.named_materials.iter() {
                        //Check if it contains an override, if it does register the handle so it's swappeg out
                        let name = named.to_string();
                        if mat_registry.contains_override(&name) {
                            mat_registry.register_swap_mat(named.to_string(), &mat.clone());
                            events.send(SwapEvent);
                        } else {
                            //If it doesn't, put it into the unprocessed materials HashMap
                            //so it can be picked up when the user (eventually) registers their main material
                            mat_registry.register_mat_for_processing(name, mat);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Event)]
pub struct SwapEvent;

/// Used to track which material handles should be swapped for a 'main-material'
/// Multiple materials can be swapped for the same main material
#[derive(Resource)]
pub struct FabMaterialOverrides<T: Material, G: Material> {
    /// Contains a map of the material name, to any materials that should be replaced by it
    pub swap_materials: HashMap<String, Vec<Handle<G>>>,
    pub main_materials: HashMap<String, Handle<T>>,

    /// Materials names that do/did not have an override when they were loaded
    pub unprocessed_materials: HashMap<String, Vec<Handle<G>>>,
}

impl<T: Material, G: Material> FabMaterialOverrides<T, G> {
    /// Creates a new, empty material index
    pub fn new() -> Self {
        Self {
            swap_materials: HashMap::new(),
            main_materials: HashMap::new(),
            unprocessed_materials: HashMap::new(),
        }
    }

    /// Register a new main material, materials loaded from GLTF's (Really anywhere) will be swapped out for the main material
    pub fn register_main_mat(&mut self, name: impl Into<String>, mat: Handle<T>) {
        let n = name.into();
        self.main_materials.insert(n.clone(), mat);

        let Some(unprocessed_mats) = self.unprocessed_materials.remove(&n) else {
            return;
        };

        for mat in unprocessed_mats {
            self.register_swap_mat(&n, &mat);
        }
    }

    /// Register a swap material. The material handle will be removed from the entity, and the main material handle will be added
    pub fn register_swap_mat(&mut self, name: impl Into<String>, mat: &Handle<G>) {
        let n = name.into();

        //Clone weak so just having this material in the array won't keep it alive / held if it's not used anywhere else
        if let Some(swaps) = self.swap_materials.get_mut(&n) {
            swaps.push(mat.clone_weak());
        } else {
            self.swap_materials.insert(n, vec![mat.clone_weak()]);
        }
    }

    /// Register a swap material. The material handle will be removed from the entity, and the main material handle will be added
    pub fn register_mat_for_processing(&mut self, name: impl Into<String>, mat: &Handle<G>) {
        let n = name.into();

        //Clone weak so just having this material in the array won't keep it alive / held if it's not used anywhere else
        if let Some(swaps) = self.unprocessed_materials.get_mut(&n) {
            swaps.push(mat.clone_weak());
        } else {
            self.unprocessed_materials.insert(n, vec![mat.clone_weak()]);
        }
    }
    /// Takes a potential swap material and checks if it is already in the registry
    pub fn get_swap_mat(&self, mat: &Handle<G>) -> Option<Handle<T>> {
        for (name, swaps) in self.swap_materials.iter() {
            if swaps.contains(mat) {
                if let Some(main_mat) = self.main_materials.get(name) {
                    return Some(main_mat.clone());
                } else {
                    warn!("Could not find main mat for swap mat with name: {}", name);
                    return None;
                }
            }
        }

        None
    }

    /// Returns whether a material should be swapped / overriden with a main material
    pub fn contains_override(&self, name: &String) -> bool {
        self.main_materials.contains_key(name)
    }
}
