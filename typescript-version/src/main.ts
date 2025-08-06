import { Client, GatewayIntentBits } from 'discord.js';
import dotenv from 'dotenv';
import { logger } from './utils/logger.js';
import { createFrameworkOptions, eventHandler } from './bot.js';

// Load environment variables (equivalent to dotenvy::dotenv().ok() in Rust)
dotenv.config();

// Main function (equivalent to #[tokio::main] async fn main() in Rust)
async function main(): Promise<void> {
    // Setup logging (equivalent to env_logger::init() in Rust)
    if (!process.env.LOG_LEVEL) {
        process.env.LOG_LEVEL = process.env.NODE_ENV === 'development' ? 'debug' : 'info';
    }

    logger.info('Today is a good day!');
    logger.info(' - Spoticord (TypeScript Edition)');

    // Validate required environment variables
    const token = process.env.BOT_TOKEN;
    if (!token) {
        logger.error('BOT_TOKEN environment variable is required');
        process.exit(1);
    }

    try {
        // Create Discord client with intents (equivalent to ClientBuilder::new in Rust)
        const client = new Client({
            intents: [
                GatewayIntentBits.Guilds,
                GatewayIntentBits.GuildMessages,
                GatewayIntentBits.GuildMembers,
                GatewayIntentBits.GuildVoiceStates, // Required for voice functionality
            ]
        });

        // Setup framework options (equivalent to .options(bot::framework_opts()) in Rust)
        createFrameworkOptions(client);

        // Setup event handlers (equivalent to .setup() in Rust)
        eventHandler(client);

        // Login to Discord (equivalent to client.start_autosharded().await in Rust)
        await client.login(token);

    } catch (error) {
        logger.error('Fatal error occurred during bot operations:', error);
        logger.error('Bot will now shut down!');
        process.exit(1);
    }
}

// Handle unhandled promise rejections and exceptions
process.on('unhandledRejection', (reason, promise) => {
    logger.error('Unhandled Rejection at:', promise, 'reason:', reason);
});

process.on('uncaughtException', (error) => {
    logger.error('Uncaught Exception:', error);
    process.exit(1);
});

// Start the application
main().catch(error => {
    logger.error('Failed to start application:', error);
    process.exit(1);
});
