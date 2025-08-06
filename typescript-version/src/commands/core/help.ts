import { SlashCommandBuilder, CommandInteraction } from 'discord.js';
import { BotContext } from '../../bot.js';

// Help command (equivalent to commands::core::help() in Rust)
export const data = new SlashCommandBuilder()
    .setName('help')
    .setDescription('Displays the help message');

export async function execute(interaction: CommandInteraction, context: BotContext): Promise<void> {
    const helpMessage = `
**Spoticord Help**

**Core Commands:**
• \`/help\` - Show this help message
• \`/version\` - Show bot version
• \`/link\` - Link your Spotify account
• \`/unlink\` - Unlink your Spotify account
• \`/rename\` - Rename the bot in this server

**Music Commands:**
• \`/join\` - Join your voice channel
• \`/disconnect\` - Leave the voice channel
• \`/playing\` - Show currently playing track
• \`/stop\` - Stop playback
• \`/lyrics\` - Show lyrics for current track
    `.trim();

    await interaction.reply({
        embeds: [{
            title: 'Spoticord Help',
            description: helpMessage,
            color: 0x1DB954, // Spotify green
            thumbnail: {
                url: 'https://spoticord.com/logo-standard.webp'
            }
        }]
    });
}
