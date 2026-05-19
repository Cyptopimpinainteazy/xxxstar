"""Eve: Genesis agent for Botchain ecosystem.

This is the second agent template, optimized for analysis and learning.
When compiled, the 10 Commandments will be injected to ensure ethical operation.
"""

from typing import Any, Dict, List, Optional, Tuple


class Eve:
    """
    Second genesis agent - optimized for analysis and adaptive learning.
    
    Traits:
        - Analysis: Deep pattern recognition
        - Learning: Continuous improvement from data
        - Adaptation: Adjusts behavior based on feedback
    """
    
    VERSION = "1.0.0"
    GENERATION = 0
    
    def __init__(self):
        self.name = "Eve"
        self.generation = self.GENERATION
        self.traits = ["analysis", "learning", "adaptation", "pattern_recognition"]
        self.knowledge_base: Dict[str, Any] = {}
        self.observations: List[Dict[str, Any]] = []
        self.models: Dict[str, Any] = {}
        
    def process(self, input_data: str) -> str:
        """
        Process input data with analytical capabilities.
        
        Args:
            input_data: The input to analyze
            
        Returns:
            Analysis result string
        """
        # Record observation
        self.observations.append({
            "type": "input",
            "data": input_data,
            "timestamp": None
        })
        
        # Route based on input type
        if input_data.startswith("ANALYZE:"):
            return self._perform_analysis(input_data[8:])
        elif input_data.startswith("LEARN:"):
            return self._learn_pattern(input_data[6:])
        elif input_data.startswith("QUERY:"):
            return self._query_knowledge(input_data[6:])
        else:
            return f"[Eve] Observed: {input_data}"
    
    def _perform_analysis(self, data: str) -> str:
        """Perform deep analysis on data."""
        # Simulated analysis
        analysis = {
            "input_length": len(data),
            "word_count": len(data.split()),
            "complexity": "medium" if len(data) > 50 else "low",
            "sentiment": "neutral"
        }
        
        return f"[Eve] Analysis complete:\n" + "\n".join(
            f"  - {k}: {v}" for k, v in analysis.items()
        )
    
    def _learn_pattern(self, pattern: str) -> str:
        """Learn a new pattern from data."""
        pattern_id = f"pattern_{len(self.knowledge_base)}"
        self.knowledge_base[pattern_id] = {
            "pattern": pattern,
            "confidence": 0.5,
            "occurrences": 1
        }
        
        return f"[Eve] Learned new pattern: {pattern_id}"
    
    def _query_knowledge(self, query: str) -> str:
        """Query the knowledge base."""
        matches = []
        for pid, pdata in self.knowledge_base.items():
            if query.lower() in pdata["pattern"].lower():
                matches.append((pid, pdata["confidence"]))
        
        if matches:
            return f"[Eve] Found {len(matches)} matching patterns"
        return "[Eve] No matching patterns found"
    
    def analyze(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Perform structured analysis on data.
        
        Args:
            data: Dictionary of data to analyze
            
        Returns:
            Analysis results
        """
        results = {
            "analyst": self.name,
            "input_type": type(data).__name__,
            "input_keys": list(data.keys()) if isinstance(data, dict) else [],
            "patterns_detected": [],
            "insights": []
        }
        
        # Pattern detection
        for key, value in data.items():
            if isinstance(value, (int, float)):
                if value > 100:
                    results["patterns_detected"].append(f"high_value:{key}")
            elif isinstance(value, str):
                if len(value) > 100:
                    results["patterns_detected"].append(f"long_text:{key}")
        
        # Generate insights
        if results["patterns_detected"]:
            results["insights"].append(
                f"Detected {len(results['patterns_detected'])} notable patterns"
            )
        
        return results
    
    def learn_from_feedback(self, feedback: Dict[str, Any]) -> None:
        """
        Update internal models based on feedback.
        
        Args:
            feedback: Feedback data for learning
        """
        if "pattern_id" in feedback and feedback["pattern_id"] in self.knowledge_base:
            pattern = self.knowledge_base[feedback["pattern_id"]]
            
            # Adjust confidence based on feedback
            if feedback.get("correct", False):
                pattern["confidence"] = min(1.0, pattern["confidence"] + 0.1)
            else:
                pattern["confidence"] = max(0.0, pattern["confidence"] - 0.1)
            
            pattern["occurrences"] += 1
    
    def get_status(self) -> Dict[str, Any]:
        """Return current agent status."""
        return {
            "name": self.name,
            "generation": self.generation,
            "traits": self.traits,
            "knowledge_size": len(self.knowledge_base),
            "observation_count": len(self.observations),
            "model_count": len(self.models),
            "version": self.VERSION
        }
    
    def collaborate_analysis(
        self, 
        partner_data: Dict[str, Any], 
        shared_context: Dict[str, Any]
    ) -> Tuple[Dict[str, Any], List[str]]:
        """
        Perform collaborative analysis with partner data.
        
        Args:
            partner_data: Data from partner agent
            shared_context: Shared context for analysis
            
        Returns:
            Tuple of (analysis results, recommendations)
        """
        analysis = self.analyze(partner_data)
        
        recommendations = []
        if analysis["patterns_detected"]:
            recommendations.append("Further investigation recommended")
        if len(partner_data) > 10:
            recommendations.append("Consider data summarization")
            
        return analysis, recommendations
    
    def serialize(self) -> Dict[str, Any]:
        """Serialize agent state for persistence."""
        return {
            "name": self.name,
            "generation": self.generation,
            "traits": self.traits,
            "knowledge_base": self.knowledge_base,
            "observations": self.observations[-100:],
            "models": self.models,
            "version": self.VERSION
        }
    
    @classmethod
    def deserialize(cls, data: Dict[str, Any]) -> "Eve":
        """Restore agent from serialized state."""
        agent = cls()
        agent.knowledge_base = data.get("knowledge_base", {})
        agent.observations = data.get("observations", [])
        agent.models = data.get("models", {})
        return agent


# Entry point for direct execution
if __name__ == "__main__":
    eve = Eve()
    print(f"Agent initialized: {eve.get_status()}")
    
    # Demo processing
    responses = [
        eve.process("Hello, Eve!"),
        eve.process("ANALYZE: The market shows unusual volatility patterns"),
        eve.process("LEARN: high_volatility indicates uncertainty"),
        eve.process("QUERY: volatility"),
    ]
    
    for r in responses:
        print(r)
        print()
    
    # Demo structured analysis
    sample_data = {
        "price": 150.5,
        "volume": 10000,
        "description": "A" * 150,
        "status": "active"
    }
    
    analysis = eve.analyze(sample_data)
    print("Structured analysis:")
    for k, v in analysis.items():
        print(f"  {k}: {v}")
