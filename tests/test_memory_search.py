import tempfile
from pathlib import Path

from debot.agent.memory import search_memory, MemoryStore


def test_memory_search_builds_index_and_finds_results():
    with tempfile.TemporaryDirectory() as td:
        ws = Path(td)
        mem = ws / "memory"
        mem.mkdir()

        # long-term memory
        (mem / "MEMORY.md").write_text(
            "Deployed service X to production on 2026-01-15. Manual rollback performed.",
            encoding="utf-8",
        )

        # daily note
        (mem / "2026-02-01.md").write_text(
            "Meeting notes: discussed deployment and rollout plan to production.", encoding="utf-8"
        )

        store = MemoryStore(ws)
        # explicit build
        count = store.build_index()
        assert count > 0

        results = search_memory(ws, "deploy production", max_results=3)
        assert isinstance(results, list)
        assert len(results) > 0
        assert any(
            "deploy" in r["snippet"].lower() or "deployed" in r["snippet"].lower() for r in results
        )
