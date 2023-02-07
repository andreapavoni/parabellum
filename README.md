# Parabellum

An attempt to make a Travian 3.x clone written in Rust.

## WIP Roadmap

- [ ] App
  - [ ] commands
  - [ ] events
  - [ ] processors
  - [ ] queries
- [ ] Db
  - [ ] generic db interface
  - [ ] ormlite and sqlite integration
  - [ ] db models and integration with domain models
  - [ ] queries
- [ ] Api
  - [ ] app integration
  - [ ] endpoints
  - [ ] auth (?)
- [ ] Game
  - [ ] refine domain models and business logic
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

### Tasks workflow

#### Enqueue

```
API(request)
-> APP[CMD_ROUTER() -> CMD(validate, exec) -> TASK]
-> DB(write task)
```

#### Execution

```
APP -> DB(read tasks)
-> APP[TASK_RUNNER(task) -> EVENT]
-> DB(write event)
-> APP[PROCESSOR(update state)]
-> DB(write state)
```

### Actions workflow

```
API(request)
-> APP[CMD_ROUTER() -> CMD(validate, exec) -> EVENT]
-> DB(write event)
-> APP[PROCESSOR(update state)]
-> DB(write state)
-> API(response)
```

### Reads

```
API(request)
-> APP[QUERY_ROUTER() -> QUERY(validate, exec)]
-> DB(read data)
-> API(response)
```
