# Parabellum

![Rust](https://img.shields.io/badge/min%20rust-1.85-green.svg)
[![CI/CD Pipeline](https://github.com/andreapavoni/parabellum/actions/workflows/ci.yml/badge.svg)](https://github.com/andreapavoni/parabellum/actions/workflows/ci.yml)
![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)

Parabellum is an attempt to build a (yet another!) modern, fast, and open-source MMORPG inspired by the classic game Travian 3.x.

This project is for those who love the deep strategy and community of the original but want an alternative built on a modern tech stack. The goal is to create a lightweight, easy-to-deploy server that's completely free from pay-to-win mechanics.

The project is still in its early stages, but the foundations are solidifying every day. It's now at a point where contributions are very welcome to help shape the game.

**HEADS UP!** Parabellum is under heavy development and is **not yet playable**. Many core mechanics are being built, but it's not a complete game (yet).

---

## Project Goals

* **Core Travian Experience**: Replicate the core 80-90% of the game mechanics (building, resource management, troops, attacks, alliances).
* **Fast, Robust & Lightweight**: Use Rust to create a high-performance server that's easy for anyone to run. Code has unit and integration tests to ensure everything is working as expected.
* **No "Pay-to-Win"**: This is a non-negotiable. This project is for the love of the game, not for predatory monetization.
* **Modern Stack**: Intentionally skipping outdated features like in-game forums or chats, assuming players will use modern tools like Discord.
* **Open Source**: Create a community-driven project that can be forked, modified, and learned from.

---

## Quick Start

Want to get the server running locally? Here’s how.

**Prerequisites:**
* Rust (>= 1.85)
* Docker & Docker Compose
* `sqlx-cli` (run `cargo install sqlx-cli --no-default-features --features postgres`)

**Steps:**

1.  **Clone the repo:**
    ```sh
    git clone https://github.com/andreapavoni/parabellum.git
    cd parabellum
    ```

2.  **Set up environment:**
    ```sh
    # Copy the sample .env file
    cp .env.sample .env
    ```
    (You shouldn't need to modify this for local dev).

3.  **Start database:**
    ```sh
    docker-compose up -d db
    ```

4.  **Create databases:**
    ```sh
    # This sets up the dev AND test databases
    ./setup_db.sh
    ```

5. **(optional) Run app in docker:**
   ```sh
   docker-compose up -d app
   ```

6.  **(optional) Run tests:**
    ```sh
    cargo test --release -- --test-threads=1
    ```
    _Note: use 1 thread only, to avoid issues in tests setup that weren't solved yet._

7.  **Run the server:**
    ```sh
    cargo run --release
    ```

  From now, you can go to `http://localhost:8080` and see the progress.

---

## Feature Roadmap

Here's a high-level tracker of what's working, what's in progress, and what's still to do.

### Implemented
- [x] **Core Architecture**: A clean, command-based application structure.
- [x] **Database**: A repository pattern for atomic database transactions.
- [x] **Job System**: An async, persistent job queue.
- [x] **Game Data**: All static data for buildings, units, tribes, and smithy upgrades is defined.
- [x] **Player**: Player registration.
- [x] **Village**: Initial village founding.
- [x] **Resources**: Passive resource generation (the "tick") based on building levels and server speed.
- [x] **Population**: Population calculation.
- [x] **Building**: Full command and job cycle for starting and completing construction.
- [x] **Unit Training**: Full command and job cycle for training a queue of units. Only barracks at the moment.
- [x] **Research**: Full command and job cycle for Academy and Smithy research.
- [x] **Battle**: Core battle logic (attacker vs. defender calculation) is implemented.
- [x] **Attack Cycle**: Full "Attack" -> "Battle" -> "Army Return" job chain.
- [x] **Battle Features**: Ram/Catapult damage and resource bounty calculation.
- [x] **Merchants**: Marketplace offers, sending resources between villages.
- [x] **Server Speed**: Full support for different server speeds influencing times and stocks/merchants capacities.
- [x] **Building Upgrades/Downgrades**: Upgrading/Downgrading buildings, also considering MainBuilding levels and server speed as well.
- [x] **Reinforcements**: Sending troops to support other villages.
- [x] **Scouting**: The "Scout" attack type (logic exists, but no command/job).
- [x] **Unit Training**: support for all units types in their related buildings.
- [x] **Heroes**: Hero model and basic bonus logic exists, but they are not yet integrated into armies or battles.
- [x] **Users and Auth**: Login/register/logout, needs password recovery
- [x] **World Map bootsrap**: Automatic bootstrap of the game map at first run.

### In Progress
- [ ] **i18n**: add translations in several languages.
- [ ] **API / UI**: Getting the minimal viable views to navigate the game
  - [x] Layout, basic navbar
  - [x] Login, Register
  - [x] Village Overview (Resources + Buildings)
    - [x] Building queue
  - [x] Generic building (info + add/upgrade)
  - [ ] Special buildings (info + specific actions)
    - [ ] Barracks, Stable, Workshop, etc...
    - [ ] Academy (research units)
    - [ ] Smithy (upgrade units)
    - [ ] Rally Point:
      - [ ] send troops
      - [ ] view troop movements (ongoing/incoming attacks/raids/reinforcements/army returns)
      - [ ] view stationed troops
    - [ ] Merchant, Marketplace
      - [ ] Send resources to a village
      - [ ] Sell/buy resources
    - [ ] Hero Mansion (hero stuff)
    - [ ] Castle/Residence (train settlers, expansion slots, culture points)
    - [ ] Town Hall (small/big party)
    - [ ] Main Building (downgrades)
  - [ ] Map

### ToDo (Not Started)
- [ ] **Reports**: reports for armies/merchants.
- [ ] **Alliances**: Creating and managing alliances.
- [ ] **Expansion**: Training settlers, tracking culture points, founding new villages (command exists, but not settlers training), and conquering.
- [ ] **Oases**: Capturing and managing oases (models exist, logic does not). Nature troops in free oases.
- [ ] **User Password recovery**: using email? Switching to OAuth?
- [ ] **Admin UI**: a minimal dashboard to manage the game.
- [ ] **End Game**: Wonder of the World, Natars, etc.
- [ ] **Help/Manual**: to learn.

---

## Project Structure

The project is structured as a Cargo workspace with several distinct crates:

* `parabellum_server`: The main binary executable. This is the entry point that ties everything together.

### Infrastructure
These packages provide the necessary tools to make the system working. They provide data persistence, communication interfaces, and they can be changed or added independently.

* `parabellum_web`: The Web UI. All the HTTP communication and web templates can be found here.
* `parabellum_db`: The database layer. This provides the concrete implementation of the database repositories (using `sqlx` and Postgres).

### Domain
These packages define the whole game engine. There aren't infrastructure details (like database, http server, etc...). Instead, there are static data (units, costs, times), game rules, validations, etc...

* `parabellum_app`: The application layer. This is the "brain" of the project. It contains all the commands, queries, and handlers that orchestrate the game logic. It also manages the Job Queue system.
* `parabellum_game`: The core domain layer. This crate knows *nothing* about databases or web servers. It contains the pure game rules, models (Village, Army, Building), and logic (e.g., `battle.rs`).
* `parabellum_core`: A shared crate for common code.
* `parabellum_types`: Shared, simple data structures that are used by all other crates to avoid circular dependencies.

---

## How to Contribute

Contributions are very welcome! Since the project is in the early stages, things are still very flexible.

1.  **Find something to work on**:
    * Look at the `ToDo` list in the roadmap above.
    * Pick an `In Progress` item and help finish it.
    * Find a bug or a missing calculation.
    * Help improve documentation or add more tests.
2.  **Get in touch**:
    * For now, the best way is to **open an Issue** on GitHub.
    * Describe what you'd like to work on or the bug you've found.
    * We can discuss the best approach there before you start coding.
3.  **Submit a Pull Request**:
    * Create a PR with your changes, and we'll review it together!

Don't worry about "doing it wrong." The most important thing is to get involved!

---

## Credits

This project wouldn't be possible without the incredible work done by the [TravianZ project](https://github.com/Shadowss/TravianZ) and [Kirilloid's work](https://github.com/kirilloid/travian) on detailing the game mechanics.

## License

Parabellum is open-source software licensed under the **MIT License**.


## Copyright

A [pavonz](https://pavonz.com) joint. (c) 2023-2025.
