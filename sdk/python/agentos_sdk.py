"""AgentOS Python SDK -- Lightweight client for the AgentOS REST API."""
import requests
from typing import Optional


class AgentOS:
    def __init__(self, host: str = "http://localhost:8080", api_key: str = ""):
        self.host = host.rstrip("/")
        self.api_key = api_key
        self.session = requests.Session()
        self.session.headers.update({
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}"
        })

    def health(self) -> dict:
        return self.session.get(f"{self.host}/health").json()

    def status(self) -> dict:
        return self.session.get(f"{self.host}/v1/status").json()

    def send_task(self, text: str) -> dict:
        return self.session.post(f"{self.host}/v1/message", json={"text": text}).json()

    def get_task(self, task_id: str) -> dict:
        return self.session.get(f"{self.host}/v1/task/{task_id}").json()


if __name__ == "__main__":
    agent = AgentOS(api_key="aos_test")
    print(agent.health())
