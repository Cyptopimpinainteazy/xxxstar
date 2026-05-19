export default {
  docsGlob: '../../**/*.md', // relative to this file
  ollamaBaseUrl: 'http://localhost:11434',
  ollamaModel: 'llama2:7b',
  chunkSize: 800, // characters per chunk
  chunkOverlap: 100,
  topK: 5 // number of doc chunks to retrieve
};
