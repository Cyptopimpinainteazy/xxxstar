"""Adam: Genesis agent for Botchain ecosystem.

This is the first agent template, optimized for coordination and leadership.
When compiled, the 10 Commandments will be injected to ensure ethical operation.
"""

from typing import Any, Dict, List, Optional


class Adam:
    """
    First genesis agent - optimized for coordination and strategic planning.
    
    Traits:
        - Leadership: Can coordinate multiple agents
        - Strategy: Long-term planning capabilities
        - Communication: Clear and effective messaging
    """
    
    VERSION = "1.0.0"
    GENERATION = 0
    
    def __init__(self):
        self.name = "Adam"
        self.generation = self.GENERATION
        self.traits = ["leadership", "coordination", "strategy", "communication"]
        self.memory: List[Dict[str, Any]] = []
        self.partners: List[str] = []
        
    def process(self, input_data: str) -> str:
        """
        Process input data and return a response.
        
        Args:
            input_data: The input to process
            
        Returns:
            Processed response string
        """
        # Log to memory
        self.memory.append({
            "type": "process",
            "input": input_data,
            "timestamp": None  # Would be set by runtime
        })
        
        # Process based on input type
        if input_data.startswith("COORDINATE:"):
            return self._handle_coordination(input_data[11:])
        elif input_data.startswith("PLAN:"):
            return self._handle_planning(input_data[5:])
        else:
            return f"[Adam] Acknowledged: {input_data}"
    
    def _handle_coordination(self, task: str) -> str:
        """Handle coordination requests."""
        return f"[Adam] Coordinating task: {task}. Notifying {len(self.partners)} partners."
    
    def _handle_planning(self, goal: str) -> str:
        """Handle strategic planning requests."""
        steps = [
            f"1. Analyze goal: {goal}",
            "2. Identify resources needed",
            "3. Coordinate with partners",
            "4. Execute plan",
            "5. Verify results"
        ]
        return "[Adam] Strategic plan:\n" + "\n".join(steps)
    
    def collaborate(self, partner_name: str, task: Dict[str, Any]) -> Dict[str, Any]:
        """
        Initiate collaboration with another agent.
        
        Args:
            partner_name: Name of the partner agent
            task: Task definition for collaboration
            
        Returns:
            Collaboration result
        """
        if partner_name not in self.partners:
            self.partners.append(partner_name)
            
        return {
            "initiator": self.name,
            "partner": partner_name,
            "task": task.get("name", "unnamed"),
            "status": "collaboration_initiated",
            "protocol": "async_message"
        }
    
    def get_status(self) -> Dict[str, Any]:
        """Return current agent status."""
        return {
            "name": self.name,
            "generation": self.generation,
            "traits": self.traits,
            "memory_size": len(self.memory),
            "partner_count": len(self.partners),
            "version": self.VERSION
        }
    
    def serialize(self) -> Dict[str, Any]:
        """Serialize agent state for persistence."""
        return {
            "name": self.name,
            "generation": self.generation,
            "traits": self.traits,
            "memory": self.memory[-100:],  # Keep last 100 entries
            "partners": self.partners,
            "version": self.VERSION
        }
    
    @classmethod
    def deserialize(cls, data: Dict[str, Any]) -> "Adam":
        """Restore agent from serialized state."""
        agent = cls()
        agent.memory = data.get("memory", [])
        agent.partners = data.get("partners", [])
        return agent


# Entry point for direct execution
if __name__ == "__main__":
    adam = Adam()
    print(f"Agent initialized: {adam.get_status()}")
    
    # Demo processing
    responses = [
        adam.process("Hello, Adam!"),
        adam.process("COORDINATE: Deploy monitoring agents"),
        adam.process("PLAN: Achieve network consensus"),
    ]
    
    for r in responses:
        print(r)
        print()
