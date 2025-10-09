# Parabellum

An attempt to make a Travian 3.x clone written in Rust.

## Quick setup and running

Execute the following commands to try Parabellum. Depending on its stage of development,
when ran it will print on the terminal what it's happening. As of 2023-0207, it just generates
a new map of 100x100 (x4) squares with each one a randomly assigned topology (valley of different resource fields, or oasis of different bonus percentuals, like in Travian).

**NOTE:** be sure to have SQLite installed in your system.

```sh
cargo install sqlx-cli --no-default-features --features rustls,sqlite
export DATABASE_URL="sqlite://parabellum.db"
sqlx setup
cargo run
```

## WIP Roadmap

In no particular order. The ones that are being currently worked have been **<ins>highlighted in underlined bold</ins>**:

- [ ] App
  - [ ] **<ins>commands</ins>**
  - [ ] **<ins>events</ins>**
  - [ ] **<ins>processors</ins>**
  - [ ] queries
- [ ] **<ins>Db</ins>**
  - [x] generic db interface
  - [x] ormlite and sqlite integration
  - [x] db models and integration with domain models
  - [ ] queries
- [ ] Api
  - [ ] app integration
  - [ ] endpoints
  - [ ] auth (?)
- [ ] **<ins>Game</ins>**
  - [ ] **<ins>refine domain models and business logic</ins>**
  - [ ] hero
    - [ ] points system
    - [ ] health
    - [ ] train/revive
  - [ ] battles
    - [ ] battle system: conquer villages/oases
    - [ ] hero bonus/health
    - [ ] reports
      - [ ] loot
      - [ ] buildings damages
      - [ ] wall damages
      - [ ] attacker remaining army
      - [ ] village remaining army and reinforcements
      - [ ] hero points

## Overall architecture

This project has been designed and split in different components to apply some isolation between different responsibilities.

- `app` wraps the several components of the app (db connection, domain models, etc...).
- `db` implements all the interactions with the database, it implements a generic interface so that it can interact with other kinds of databases other than the actual one (SQLite).
- `game` specifies all the domain models and business logic, it tries to be as standalone as possible in respect to the other components.
- `api` implements a REST API in json, so that it's potentially possible to use different UIs.

Also, to help with the different actions, interactions and outcomes of the game, it has been applied a very simplified version of the CQRS/ES pattern: app calls _commands_ which generates _events_. The events are stored and then processed to change a state of the object, finally the object state gets persisted on db.

## FAQ

### Q: Why yet another attempt to make a Travian clone?

Why not? [TravianZ](https://github.com/Shadowss/TravianZ) is an excellent project! But it has many years on its backs (even in terms of technology and design patterns), and develpopment efforts are left back to random volunteers wanting to contribute. I always dreamed about making a Travian clone, and this is my opportunity.

### Q: What are the goals? Are you planning to make a 1:1 clone of TravianLegends/TravianZ?

In the beginning, the first goal is to get a playable game with at least 80-90% of the main features of TravianZ.

Another main goal is to make it fast and easy to deploy, that's one of the reasons why I chose Rust to implement this project, and SQLite as a database.

Some of the known features will be avoided because outdated and/or not strictly useful, in particular:

- No _Plus_ and neither _golds_ or _silvers_ :-) I don't like the PayForWin approach. Monetization is not planned yet, but when/if it will come, it will be for things will help the player to _play better_, not to have an advantage over the ones that don't pay.
- No alliance forum/chat: it made sense before 2010s, but as of today, it's just a burden, and people use other tools to communicate (Discord, slack, instant messengers...)

### Q: Where can I find a demo server?

There isn't one yet, because it's still under heavy development and there isn't anything to show, except the good much of code published here.

### Q: What about the UI? Will it be the same the players already know?

Being usability and portability a main goal fo this project, the UI will be designed to be comfortable even on small screens, so the UI will probably be very different. I'm very far from being a graphic designer, so I hope someone will jump in to help.

### Q: Will Parabellum have localized translations?

Maybe. The initial main language will be English for _ubiquity_ reasons, but I don't exclude the possibility to add more languages later if the project will gain popularity.

## Credits

It would have been nearly impossible to start this project without the efforts of many people that contributed (and still does) to [TravianZ](https://github.com/Shadowss/TravianZ) project (and its many forks around the web). Also [Kirilloid's work](https://github.com/kirilloid/travian) has been fundamental to apply the battle system formulas in this project.

## Copyright

A [pavonz](https://pavonz.com) joint. (c) 2023.
