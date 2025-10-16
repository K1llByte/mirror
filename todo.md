# In Progress
## Simple path tracer - v1.0
- [ ] Fix render task never finishing when using remote peers

# Todo
## Simple path tracer - v1.0
- [ ] Better logs (trace log latency time on each render/sync packet sent, trace log on render tile request render time)
- [ ] connect_to_peers methods should connect to all at the same time
- [ ] When sending scene sync packet avoid cloning whole scene, this will become costly later when theres gigabytes of models loaded
- [ ] Avoid sending scene on every render request, for progressive rendering this will avoid synchronizing while the scene did not change

- [ ] Benchmark (single task vs multi task vs multi task and remote nodes)
- [ ] Documentation:
  - [ ] Update README.md with:: What it is, What technologies uses, How to use,
  how it works, references and documentation
  - [ ] Code documentation

## Web client
- New option to create a web server instead of a native client.
- Webserver will provide the webassembly compiled version of egui that will communicate
with the node via websockets.

## Improved path tracer
- Diff-based scene update/synchronization between nodes
- Gltf2 scene loading
- Acceleration structure
- Scheduler that considers node latency and performance score
- Implement quadrilaterals
- Meshes
- Button to save rendered image to file

## Unsorted
- [ ] FIXME: Hardcoded 127.0.0.1 for now, will change this to a Hello handshake returning an id
- [ ] Github CI workflows
    - [ ] Create a dev branch and always active work there
    - [ ] CI that checks clippy before merging into a releases branch
    - [ ] CI that builds a gh-pages branch that deploys webassembly generated project to the web
- [ ] PeerTable should store peer data as Arc<Mutex<Peer>> instead of current approach
- [ ] Implement some image denoising algorithm such as bilateral filter

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
- [x] Solve problem with multithreading not having any performance increase
- [x] Solve image luminance not working properly, probably due to sample value
- [x] Solve problem with rendering with remote peers not being better performing
- [x] Change render_image lock to read write lock
- [x] Benchmark (single task vs multi task vs multi task and remote nodes)
- [x] Make times_sampled a field in Image (AccumulatedImage)
- [x] Fault tolerance: if fails to send, resend to queue.
- [x] Cool new scene
