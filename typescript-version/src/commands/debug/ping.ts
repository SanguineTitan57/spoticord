import { SlashCommandBuilder, CommandInteraction } from 'discord.js';
import { BotContext } from '../../bot.js';

// Debug ping command (equivalent to commands::debug::ping() in Rust)
export const data = new SlashCommandBuilder()
    .setName('ping')
    .setDescription('Replies with Pong! (Debug only)');

export async function execute(interaction: CommandInteraction, context: BotContext): Promise<void> {
    const sent = await interaction.reply({
        content: 'Pinging...',
        fetchReply: true
    });

    const latency = sent.createdTimestamp - interaction.createdTimestamp;
    const wsLatency = context.client.ws.ping;

    await interaction.editReply({
        content: `🏓 Pong!\n📶 Latency: ${latency}ms\n💓 WebSocket: ${wsLatency}ms`
    });
}
