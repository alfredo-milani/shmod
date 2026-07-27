# TODO

- [ ] Let each module define a matching cleanup: a `foo.sh` pairs with a
      `foo.unload.sh`, or defines a `_shmod_unload_foo()` function that `use`
      calls before switching. Clean, but only unloads what module authors
      explicitly wrote teardown for.
    - Should unload work per profile only, or also for individual files ?
    - What happens when no companion unload script exists? (Print an error.)

- [ ] Add setting parameter to point at a different location where bash-module are.
