# `parabellum_web`

Primary documentation now lives in Rust doc comments alongside code:

- crate/module docs: [lib.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/lib.rs), [api/mod.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/mod.rs)
- router/state: [http.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/http.rs)
- handlers: [api/auth.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/auth.rs), [api/game.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/game.rs), [api/actions.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/actions.rs), [api/buildings.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/buildings.rs)
- DTO contracts: [api/dto.rs](/Users/andrea/Code/Apps/parabellum/parabellum_web/api/dto.rs)

Use Rustdoc to browse docs:

```sh
cargo doc -p parabellum_web --no-deps --open
```
