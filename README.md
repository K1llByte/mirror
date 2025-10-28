# Mirror

![Rust](https://img.shields.io/badge/Rust-🦀-orange)
![License](https://img.shields.io/github/license/k1llbyte/mirror)

A physically based path tracer designed to render locally or using a P2P overlay network for distributed rendering.

![](images/showcase.png)

## Getting Started
### Install

Clone repository
```sh
git clone git@github.com:K1llByte/mirror.git
```
Build and install binary
```sh
cargo install --path . --bin mirror
```

### Run

If no `--scene` argument is supplied cornell scene will be used by default.

> [!NOTE]
> Currently only soome predefined scenes are available to use. In the future there will be a glft2 format loader to allow any costumscenes to be rendered.

```sh
mirror --scene cornell
```

## Configuring Network

Create a `config.toml` file and specify the host binding address and some bootstrap peer addresses.

```toml
host = "0.0.0.0:2020"
bootstrap_peers = [
    "192.168.1.120:2020",
    # ...
]
```

Run `mirror` and specify the config.

```sh
mirror --config config.toml
```

<!--
## Refereces
- https://pbr-book.org/4ed/contents
- https://raytracing.github.io/books/RayTracingInOneWeekend.html
- https://raytracing.github.io/books/RayTracingTheNextWeek.html
- https://pragprog.com/titles/jbtracer/the-ray-tracer-challenge/
- https://alain.xyz/blog/ray-tracing-denoising
- https://www.graphics.cornell.edu/online/box/data.html
- https://raytracing.github.io/books/RayTracingTheRestOfYourLife.html
- https://research.nvidia.com/publication/2017-07_interactive-reconstruction-monte-carlo-image-sequences-using-recurrent
- https://helpx.adobe.com/content/dam/help/en/substance-3d/documentation/s3d/files/225969597/225969613/1/1647027222890/adobe-standard-material-specification.pdf
- https://graphics.pixar.com/library/RendermanTog2018/paper.pdf
-->