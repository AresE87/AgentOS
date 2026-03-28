"""Node discovery via mDNS (local) and relay server (remote)."""

from __future__ import annotations

from agentos.mesh.identity import NodeProfile
from agentos.utils.logging import get_logger

logger = get_logger("mesh.discovery")

MDNS_SERVICE_TYPE = "_agentos._tcp.local."
DEFAULT_PORT = 9090


class NodeDiscovery:
    """Discovers other AgentOS nodes on the network."""

    def __init__(self, identity_node_id: str, port: int = DEFAULT_PORT) -> None:
        self._node_id = identity_node_id
        self._port = port
        self._known_nodes: dict[str, NodeProfile] = {}
        self._zeroconf: object | None = None
        self._browser: object | None = None

    async def start_mdns(self) -> None:
        """Start mDNS publishing and browsing."""
        try:
            import socket

            from zeroconf import ServiceBrowser, ServiceInfo, Zeroconf

            zc = Zeroconf()
            self._zeroconf = zc
            # Register our service
            info = ServiceInfo(
                MDNS_SERVICE_TYPE,
                f"agentos-{self._node_id}.{MDNS_SERVICE_TYPE}",
                addresses=[socket.inet_aton(socket.gethostbyname(socket.gethostname()))],
                port=self._port,
                properties={"node_id": self._node_id},
            )
            zc.register_service(info)
            logger.info("mDNS service registered: %s on port %d", self._node_id, self._port)

            # Browse for other nodes
            discovery = self

            class Listener:
                def add_service(self, zc_inst: Zeroconf, type_: str, name: str) -> None:
                    svc_info = zc_inst.get_service_info(type_, name)
                    if svc_info and svc_info.properties:
                        nid = svc_info.properties.get(b"node_id", b"").decode()
                        if nid and nid != discovery._node_id:
                            addr = ""
                            if svc_info.parsed_addresses():
                                addr = f"{svc_info.parsed_addresses()[0]}:{svc_info.port}"
                            discovery._known_nodes[nid] = NodeProfile(
                                node_id=nid,
                                display_name=nid,
                                public_key=b"",
                                capabilities=None,
                                address=addr,
                                is_online=True,
                            )
                            logger.info("Discovered node: %s at %s", nid, addr)

                def remove_service(self, zc_inst: Zeroconf, type_: str, name: str) -> None:
                    pass

                def update_service(self, zc_inst: Zeroconf, type_: str, name: str) -> None:
                    pass

            self._browser = ServiceBrowser(zc, MDNS_SERVICE_TYPE, Listener())
        except ImportError:
            logger.warning("zeroconf not installed — mDNS discovery unavailable")
        except Exception:
            logger.warning("mDNS failed to start — may not have network access")

    async def stop_mdns(self) -> None:
        if self._zeroconf:
            self._zeroconf.close()  # type: ignore[attr-defined]

    def add_node_manually(self, node_id: str, address: str) -> None:
        """Add a node by IP address."""
        self._known_nodes[node_id] = NodeProfile(
            node_id=node_id,
            display_name=node_id,
            public_key=b"",
            capabilities=None,
            address=address,
            is_online=True,
        )

    def get_known_nodes(self) -> list[NodeProfile]:
        return list(self._known_nodes.values())

    def get_online_nodes(self) -> list[NodeProfile]:
        return [n for n in self._known_nodes.values() if n.is_online]

    def mark_offline(self, node_id: str) -> None:
        if node_id in self._known_nodes:
            self._known_nodes[node_id].is_online = False

    def mark_online(self, node_id: str) -> None:
        if node_id in self._known_nodes:
            self._known_nodes[node_id].is_online = True
