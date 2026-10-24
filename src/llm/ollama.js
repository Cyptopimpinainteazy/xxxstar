/* Ollama provider helpers shared by the desktop and RAG integrations. */

function cosineSim(left, right) {
  if (!Array.isArray(left) || !Array.isArray(right)) {
    throw new TypeError('cosineSim expects two numeric arrays');
  }
  if (left.length !== right.length) {
    throw new RangeError('cosineSim vectors must have equal dimensions');
  }

  let dot = 0;
  let leftMagnitude = 0;
  let rightMagnitude = 0;
  for (let index = 0; index < left.length; index += 1) {
    const leftValue = left[index];
    const rightValue = right[index];
    if (!Number.isFinite(leftValue) || !Number.isFinite(rightValue)) {
      throw new TypeError('cosineSim vectors must contain finite numbers');
    }
    dot += leftValue * rightValue;
    leftMagnitude += leftValue * leftValue;
    rightMagnitude += rightValue * rightValue;
  }

  if (leftMagnitude === 0 || rightMagnitude === 0) {
    return 0;
  }
  return dot / Math.sqrt(leftMagnitude * rightMagnitude);
}

function normalizeThink(think, thinkLevel) {
  if (thinkLevel !== undefined) {
    if (!Number.isInteger(thinkLevel) || thinkLevel < 0) {
      throw new TypeError('thinkLevel must be a non-negative integer');
    }
    return thinkLevel;
  }
  return think === true ? true : undefined;
}

module.exports = {
  cosineSim,
  normalizeThink,
};
