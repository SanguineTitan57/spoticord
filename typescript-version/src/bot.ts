import { Client, Events, GatewayIntentBits, Collection, ActivityType } from 'discord.js';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Database } from './database/Database.js';
import { SessionManager } from './session/SessionManager.js';
import { logger } from './utils/logger.js';

// Fix __dirname for ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Type definitions (equivalent to Rust pub type declarations)
export interface BotContext {
    client: Client;
    database: Database;
    sessionManager: SessionManager;
}

export interface Command {
    data: any;
    execute: (interaction: any, context: BotContext) => Promise<void>;
}

// Extend Discord.js Client
declare module 'discord.js' {
    export interface Client {
        commands: Collection<string, Command>;
        context: BotContext;
    }
}

// Framework options equivalent (mirrors framework_opts in Rust)
export function createFrameworkOptions(client: Client): void {
    client.commands = new Collection();

    // Load commands with conditional compilation equivalent
    const commandCategories = [
        // Debug commands (only in development)
        ...(process.env.NODE_ENV === 'development' ? ['debug'] : []),
        // Core commands (always available)
        'core',
        'music'
    ];

    loadCommands(client, commandCategories);
    setupEventHandlers(client);
}

// Command loading (equivalent to commands: vec![...] in Rust)
async function loadCommands(client: Client, categories: string[]): Promise<void> {
    const foldersPath = path.join(__dirname, 'commands');

    for (const category of categories) {
        const categoryPath = path.join(foldersPath, category);

        // Skip if category folder doesn't exist
        if (!fs.existsSync(categoryPath)) {
            if (process.env.NODE_ENV === 'development') {
                logger.warn(`Command category '${category}' folder not found`);
            }
            continue;
        }

        const commandFiles = fs.readdirSync(categoryPath)
            .filter((file: string) => file.endsWith('.ts') || file.endsWith('.js'));

        for (const file of commandFiles) {
            const filePath = path.join(categoryPath, file);
            try {
                const command = await import(`file://${filePath}`);
                const cmd = command.default || command;

                if (cmd && 'data' in cmd && 'execute' in cmd) {
                    client.commands.set(cmd.data.name, cmd);
                    logger.info(`Loaded ${category} command: ${cmd.data.name}`);
                } else {
                    logger.warn(`Command at ${filePath} missing required properties`);
                }
            } catch (error) {
                logger.error(`Error loading command from ${filePath}:`, error);
            }
        }
    }
}

// Event handler setup (equivalent to event_handler setup in Rust)
function setupEventHandlers(client: Client): void {
    const eventsPath = path.join(__dirname, 'events');

    if (!fs.existsSync(eventsPath)) {
        logger.warn('Events folder not found');
        return;
    }

    const eventFiles = fs.readdirSync(eventsPath)
        .filter((file: string) => file.endsWith('.ts') || file.endsWith('.js'));

    for (const file of eventFiles) {
        const filePath = path.join(eventsPath, file);
        try {
            const event = require(filePath);
            if (event.once) {
                client.once(event.name, (...args: any[]) => event.execute(...args, client.context));
            } else {
                client.on(event.name, (...args: any[]) => event.execute(...args, client.context));
            }
            logger.info(`Loaded event: ${event.name}`);
        } catch (error) {
            logger.error(`Error loading event from ${filePath}:`, error);
        }
    }

    // Main interaction handler (equivalent to command processing in Rust)
    client.on(Events.InteractionCreate, async interaction => {
        if (!interaction.isChatInputCommand()) return;

        const command = client.commands.get(interaction.commandName);

        if (!command) {
            logger.error(`No command matching ${interaction.commandName} was found`);
            return;
        }

        try {
            await command.execute(interaction, client.context);
        } catch (error) {
            logger.error('Command execution error:', error);

            const errorMessage = 'There was an error while executing this command!';
            if (interaction.replied || interaction.deferred) {
                await interaction.followUp({ content: errorMessage, ephemeral: true });
            } else {
                await interaction.reply({ content: errorMessage, ephemeral: true });
            }
        }
    });
}

// Setup function (equivalent to pub async fn setup in Rust)
export async function setup(client: Client, database: Database): Promise<BotContext> {
    logger.info(`Successfully logged in as ${client.user?.tag}`);

    // Initialize session manager (equivalent to SessionManager::new in Rust)
    const sessionManager = new SessionManager(database);

    // Create bot context (equivalent to returning Data in Rust)
    const context: BotContext = {
        client,
        database,
        sessionManager
    };

    client.context = context;

    // Register commands conditionally (like Rust cfg attributes)
    if (process.env.NODE_ENV === 'development' && process.env.GUILD_ID) {
        // Register commands in specific guild for development (faster)
        await registerGuildCommands(client, process.env.GUILD_ID);
    } else {
        // Register commands globally for production
        await registerGlobalCommands(client);
    }

    // Start background tasks (equivalent to tokio::spawn in Rust)
    startBackgroundLoop(context);

    return context;
}

// Command registration functions
async function registerGuildCommands(client: Client, guildId: string): Promise<void> {
    try {
        const commands = Array.from(client.commands.values()).map(cmd => cmd.data);
        const guild = await client.guilds.fetch(guildId);
        await guild.commands.set(commands);
        logger.info(`Registered ${commands.length} commands in guild ${guildId}`);
    } catch (error) {
        logger.error('Failed to register guild commands:', error);
    }
}

async function registerGlobalCommands(client: Client): Promise<void> {
    try {
        const commands = Array.from(client.commands.values()).map(cmd => cmd.data);
        await client.application?.commands.set(commands);
        logger.info(`Registered ${commands.length} commands globally`);
    } catch (error) {
        logger.error('Failed to register global commands:', error);
    }
}

// Event handler function (equivalent to async fn event_handler in Rust)
export async function eventHandler(client: Client): Promise<void> {
    client.once(Events.ClientReady, async (readyClient) => {
        logger.info(`Ready! Logged in as ${readyClient.user.tag}`);

        // Set bot activity (equivalent to ctx.set_activity in Rust)
        const motd = process.env.MOTD || "Spotify music";
        readyClient.user.setActivity(motd, { type: ActivityType.Listening });

        // Initialize the bot context after ready
        const database = new Database();
        await database.connect();

        const context = await setup(readyClient, database);

        // Setup graceful shutdown (equivalent to tokio::signal::ctrl_c in Rust)
        setupGracefulShutdown(context);
    });
}

// Background loop (equivalent to async fn background_loop in Rust)
function startBackgroundLoop(context: BotContext): void {
    // Stats update interval (equivalent to tokio::time::sleep in Rust)
    const statsInterval = setInterval(async () => {
        try {
            if (process.env.STATS_ENABLED === 'true') {
                const activeCount = await context.sessionManager.getActiveSessionCount();
                logger.debug(`Active sessions count: ${activeCount}`);
                // Update stats here if you have a stats service
            }
        } catch (error) {
            logger.error('Failed to update stats:', error);
        }
    }, 60000); // 60 seconds

    // Store interval reference for cleanup
    context.client.statsInterval = statsInterval;
}

// Graceful shutdown (equivalent to signal handling in Rust)
function setupGracefulShutdown(context: BotContext): void {
    const gracefulShutdown = async (signal: string) => {
        logger.info(`Received ${signal} signal, shutting down...`);

        // Clear intervals
        if (context.client.statsInterval) {
            clearInterval(context.client.statsInterval);
        }

        // Shutdown session manager
        await context.sessionManager.shutdownAll();

        // Close database connection
        await context.database.disconnect();

        // Destroy Discord client
        context.client.destroy();

        process.exit(0);
    };

    process.on('SIGINT', () => gracefulShutdown('SIGINT'));
    process.on('SIGTERM', () => gracefulShutdown('SIGTERM'));
}

// Extend Client interface for our custom properties
declare module 'discord.js' {
    export interface Client {
        statsInterval?: NodeJS.Timeout;
    }
}
