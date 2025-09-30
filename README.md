# Parabellum

An attempt to make a Travian 3.x clone written in Rust.

## Quick setup and running

Execute the following commands to try Parabellum. Depending on its stage of development,
when ran it will print on the terminal what it's happening. As of 2023-0207, it just generates
a new map of 100x100 (x4) squares with each one a randomly assigned topology (valley of different resource fields, or oasis of different bonus percentuals, like in Travian).

```sh
cp .env.sample .env
docker-compose up -d

cargo install diesel-cli --no-default-features --features postgres
diesel migration run
cargo run
```


## FAQ

### Q: Is it usable yet?
NO. It's still in its early stages, it has some isolated partially working parts, nothing that can be considered done yet.

### Q: Why yet another attempt to make a Travian clone?

Why not? [TravianZ](https://github.com/Shadowss/TravianZ) is an excellent project! But it has many years on its backs (even in terms of technology and design patterns), and develpopment efforts are left back to random volunteers wanting to contribute. I always dreamed about making a Travian clone, and this is my opportunity.

### Q: What are the goals? Are you planning to make a 1:1 clone of TravianLegends/TravianZ?

In the beginning, the first goal is to get a playable game with at least 80-90% of the main features of TravianZ.

Another main goal is to make it fast and easy to deploy, that's one of the reasons why I chose Rust to implement this project.

Some of the known features will be avoided because outdated and/or not strictly useful, in particular:

- No _Plus_ and neither _golds_ or _silvers_ :-) I don't like the PayForWin approach. Monetization is not planned yet, but when/if it will come, it will be for things that will help the player to _play better_, not to have an advantage over the ones that don't pay.
- No alliance forum/chat: it made sense before 2010s, but as of today, it's just a burden, and people use other tools to communicate (Discord, slack, instant messengers...)

### Q: Where can I find a demo server?

There isn't one yet, because it's still under heavy development and there isn't anything to show yet, except the bunch of code published here.

### Q: What about the UI? Will it be the same the players already know?

Being usability and portability a main goal fo this project, the UI will be designed to be comfortable even on small screens, so the UI will probably be very different. I'm very far from being a graphic designer, so I hope someone will jump in to help.

### Q: Will Parabellum have localized translations?

Maybe. The initial main language will be English for _ubiquity_ reasons, but I don't exclude the possibility to add more languages later if the project will gain popularity.

## Credits

It would have been nearly impossible to start this project without the efforts of many people that contributed (and still does) to [TravianZ](https://github.com/Shadowss/TravianZ) project (and its many forks around the web). Also [Kirilloid's work](https://github.com/kirilloid/travian) has been fundamental to apply the battle system formulas in this project.

## Copyright

A [pavonz](https://pavonz.com) joint. (c) 2023-2025.
