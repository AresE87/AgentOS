"""Tests for AOS-062 Node Discovery."""

from __future__ import annotations

import sys
from typing import TYPE_CHECKING

from agentos.mesh.discovery import NodeDiscovery

if TYPE_CHECKING:
    import pytest


def test_add_node_manually() -> None:
    """Manually added node appears in known nodes."""
    disc = NodeDiscovery("node-aaa111")
    disc.add_node_manually("node-bbb222", "192.168.1.10:9090")

    known = disc.get_known_nodes()
    assert len(known) == 1
    assert known[0].node_id == "node-bbb222"
    assert known[0].address == "192.168.1.10:9090"
    assert known[0].is_online is True


def test_get_online_nodes() -> None:
    """Only online nodes are returned by get_online_nodes."""
    disc = NodeDiscovery("node-aaa111")
    disc.add_node_manually("node-bbb222", "192.168.1.10:9090")
    disc.add_node_manually("node-ccc333", "192.168.1.11:9090")

    disc.mark_offline("node-bbb222")

    online = disc.get_online_nodes()
    assert len(online) == 1
    assert online[0].node_id == "node-ccc333"


def test_mark_offline() -> None:
    """Node can be marked offline."""
    disc = NodeDiscovery("node-aaa111")
    disc.add_node_manually("node-bbb222", "192.168.1.10:9090")

    disc.mark_offline("node-bbb222")

    known = disc.get_known_nodes()
    assert known[0].is_online is False


async def test_mdns_not_available(monkeypatch: pytest.MonkeyPatch) -> None:
    """When zeroconf is not importable, start_mdns warns but does not crash."""
    # Temporarily remove zeroconf from available modules
    original = sys.modules.get("zeroconf")
    monkeypatch.setitem(sys.modules, "zeroconf", None)  # type: ignore[arg-type]

    disc = NodeDiscovery("node-aaa111")
    # Should not raise
    await disc.start_mdns()
    assert disc.get_known_nodes() == []

    # Restore
    if original is not None:
        monkeypatch.setitem(sys.modules, "zeroconf", original)
