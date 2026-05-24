# EdgeCrab demo workspace

Local sandbox for interactive feature verification. Tests run with `EDGECRAB_HOME`
pointing at a temp dir under `demo/.edgecrab-home/` so production state is untouched.

## Persistent goals

```bash
./demo/persistent-goals/run.sh
```

Runs mock-provider regression tests, then optionally a live Copilot (`copilot/gpt-5-mini`) E2E
when GitHub Copilot credentials are available.
