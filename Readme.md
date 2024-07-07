# Bevy Prefabbed

## Goals

- Make it easy to integrate components onto nodes in a loaded GLTF scene
- All transforms apply to scenes
- Either transform the loaded scene onLoad or transform the scene onSpawn, or both. Only one loadTransform per-Scene asset

 ## Prefabs
 Prefabs modify the loaded GLTF scene directly _before_ the Scene is spawned in the world. Runs once on Load
 
 ## Postfabs
 Postfabs are operations done _after_ a scene has spawned ala bevy_scene_hook
 Postfabs are intended to be used to find and modify the Root of the spawned entity, or it's children
 using operators
 
 Example Postfab operators:
 
 - GetChildWithComponent
 - GetChildByName
 - RemoveComponentFromChildren
 - Children
 - AddComponentToChildrenIfComponent