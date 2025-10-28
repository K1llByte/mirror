# In Progress
- [ ] On RenderTileRequest, spawn as many tasks as render tipe requests
- [ ] Fix "if nan then choose a vector" in materials
- [ ] Fix hardcoded 127.0.0.1, change this to a Hello handshake returning an id

# Todo
## Simple path tracer - v1.0
- [ ] Scene selector
- [ ] Avoid sending scene on every render request, for progressive rendering this will avoid synchronizing while the scene did not change
- [ ] Direct light sampling
- [ ] BSDF refactor
- [ ] Textures
- [ ] Transform
    - [ ] Translation
    - [ ] Rotation
    - [ ] Scale
- [ ] On RenderTileRequest, spawn as many tasks as render tile requests
- [ ] Each peer sends its max batch size in the Hello handshake packet

- [ ] Documentation:
  - [ ] Update README.md with: What it is, What technologies uses, How to use,
  how it works, references and documentation
  - [ ] Code documentation

## Web client
- [ ] New egui interface that compiles to webassembly
- [ ] Experiment with tokio for webassembly
- New option to create a web server instead of a native client.
- Webserver will provide the webassembly compiled version of egui that will communicate
with the node via websockets.

## Improved scheduler
- Backend abstraction
    - Use rayon crate's thread pool for the rendering cpu bound work
    - The protocol part will still use tokio runtime
- Scheduler that considers node latency and performance score
- Scheduler that distributes workload based on local performance

## Improved path tracer
- Explore new BRDF models (Burley, Oren nayar, Chan, Callisto, GGX, Trowbridge-Reitz)
- Diff-based scene update/synchronization between nodes
- Gltf2 scene loading
- Meshes
- Volumes

## Unsorted
- [ ] connect_to_peers methods should connect to all at the same time

- [ ] Github CI workflows
    - [ ] Create a dev branch and always active work there
    - [ ] CI that checks unit tests
    - [ ] CI that checks clippy before merging into a releases branch
    - [ ] CI that builds a gh-pages branch that deploys webassembly generated project to the web
- [ ] PeerTable should store peer data as Arc<Mutex<Peer>> instead of current approach
- [ ] Implement some image denoising algorithm such as bilateral filter
- [ ] Fix problem that when sample count is low (1 sample) the light seems to be darker
- [ ] Fix non rendering face of geometry is rendering as opaque color when theres no light
- [ ] When sending scene sync packet avoid cloning whole scene, this will become costly later when theres gigabytes of models loaded

____________________________________________________________________________________

# Done
## P2P Overlay Network
- [x] Basic scene with just camera and spheres
- [x] Basic backend that receives connections and creates a task
- [x] Peer bootstrapping, connect to a list of predefined peers
- [x] Basic Packet with Ping message and manual binary serialization
- [x] Peer table, when listener receives connection, adds stream to table
- [x] Peer discovery. When a peer connects, send all currently connected peers
    - [x] Prevent self connection

## P2P Scene synchronization and workload dispatch
- [x] Scene data with spheres only
- [x] Use bincode for serialization
- [x] Remote and local render_tile dispatch
- [x] Disable Render button while its still rendering and say it is loading
- [x] When a remote task fails, send back to the queue again for other tasks
to work on it
    - [x] Test this with a malicious mirror program

## Simple path tracer - v1.0
- [x] Change how camera works, it should know the image size, instead specify normal camera parameters such as fov and aspect ratio
- [x] Implement framebuffer resize
- [x] Clear button
- [x] Fix UI update problem when progressive rendering is on
- [x] Solve blocking show_render_image
- [x] Solve image luminance not working properly, probably due to sample value
- [x] Solve problem with rendering with remote peers not being better performing
- [x] Change render_image lock to read write lock
- [x] Benchmark (single task vs multi task vs multi task and remote nodes)
- [x] Make times_sampled a field in Image (AccumulatedImage)
- [x] Fault tolerance: if fails to send, resend to queue.
- [x] Cool new scene
- [x] Fix render task never finishing when using remote peers
- [x] Write Aabb tests
- [x] Change project structure
- [x] Fix Aabb inverse intersection
- [x] Better logs:
    - [x] Remove Listen/Bootstrap tags
    - [x] Remove debug logs
    - [x] trace logs latency time on each render/sync packet sent,
    - [x] trace log on render tile request render time
- [x] Add render time peer spent rendering in the RenderTileResponse
- [x] BVH
    - [x] Reimplement tmin tmax in Ray (This will have performance improvements
        - [x] Reimplement tmin tmax in Sphere
        - [x] Reimplement tmin tmax in Aabb
        - [x] Reimplement tmin tmax in BvhNode
    - [x] Create a new ray everytime we change tmin and tmax
    since intersection with aabb's will early return for far away boxes)
    - [x] Use comparison axis with longest extent of node aabb
- [x] Quads
- [x] Fix Nan ray direction in diffuse material
- [x] Material emissive lights
- [x] Cornell box scene
- [x] Button to save rendered image to file
- [x] Cuboid
- [x] Send multiple tiles to render in the same render tile request packet
