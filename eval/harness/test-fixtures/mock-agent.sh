#!/bin/bash
# Mock agent that writes a known file change
# Used for integration testing the agent invocation system

echo "Mock agent starting..."
echo "Prompt received: $1"

# Create the expected output file
cat > hello.py << 'PYEOF'
def greet(name):
    return f"Hello, {name}!"
PYEOF

echo "jig run recipes/create-test/recipe.yaml --vars '{\"name\": \"test\"}'"
echo "Mock agent done."
