# X3 App Store

The X3 App Store is an automated platform that discovers, tests, and deploys crypto applications while implementing a profit-sharing model. The platform automatically searches GitHub for complete crypto projects (airdrops, farming, mining, etc.), tests them in a sandbox environment, and adds them to the app store. Users who sign up through X3 receive 50% of the profits, while the other 50% goes to the treasury in the native token.

## Features

- **Automated GitHub Discovery**: Continuously searches GitHub for complete crypto projects
- **Sandbox Testing**: Automatically tests projects in a secure sandbox environment
- **Profit Sharing**: 50/50 split between users and treasury
- **Automatic Treasury Management**: Supports all tokens and automatically adds new airdrop tokens
- **Reward Claiming**: Users can claim initial airdrops and periodic rewards
- **App Store Integration**: Automatically adds approved apps to the X3 app store

## Architecture

The X3 App Store consists of several microservices:

- **Backend API**: Main application server with REST API
- **Frontend**: User interface for browsing and claiming rewards
- **GitHub Scraper**: Automated GitHub project discovery
- **Treasury Manager**: Handles token management and profit distribution
- **App Store Manager**: Manages app approval and deployment
- **Sandbox**: Secure testing environment for new projects

## Quick Start

### Prerequisites

- Node.js 18+
- MongoDB
- Docker (optional)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/Cyptopimpinainteazy/x3-chain.git
   cd x3-app-store
   ```

2. Install dependencies:
   ```bash
   npm install
   npm run install-all
   ```

3. Set up environment variables:
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. Start the development environment:
   ```bash
   npm run dev
   ```

### Using Docker

1. Build and start all services:
   ```bash
   docker-compose -f docker-compose.dev.yml up -d
   ```

2. Access the application:
   - Frontend: http://localhost:3001
   - Backend API: http://localhost:3000

## API Endpoints

### Projects
- `GET /api/projects` - List all projects
- `POST /api/projects/search` - Search for projects
- `POST /api/projects/:id/sandbox` - Send project to sandbox
- `POST /api/projects/:id/test` - Send project to testing
- `POST /api/projects/:id/approve` - Approve project
- `POST /api/projects/:id/reject` - Reject project

### Users
- `POST /api/users/register` - Register new user
- `GET /api/users/me` - Get current user
- `POST /api/rewards/claim` - Claim rewards

### Health
- `GET /api/health` - Health check

## Deployment

The X3 App Store supports multiple deployment environments:

### Development
```bash
./deploy.sh dev
```

### Staging
```bash
./deploy.sh staging
```

### Production
```bash
./deploy.sh prod
```

## Environment Variables

### Backend
- `MONGODB_URI` - MongoDB connection string
- `GITHUB_TOKEN` - GitHub API token
- `PORT` - Backend port (default: 3000)
- `JWT_SECRET` - JWT secret key
- `SESSION_SECRET` - Session secret key

### Frontend
- `API_URL` - Backend API URL
- `PORT` - Frontend port (default: 3001)

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License

## Support

For support and questions, please open an issue in the repository.

## Security

The X3 App Store implements security best practices including:
- JWT authentication
- Input validation
- Secure sandbox environment
- Regular security audits