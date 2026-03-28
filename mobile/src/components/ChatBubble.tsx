// ---------------------------------------------------------------------------
// AgentOS Mobile -- Chat bubble component
// ---------------------------------------------------------------------------

import React from 'react';
import { StyleSheet, Text, View } from 'react-native';
import type { ChatMessage } from '../types/api';
import { colors, radii, spacing, typography } from '../theme';

interface ChatBubbleProps {
  message: ChatMessage;
}

/** Detect fenced code blocks (```...```) and render them in a mono box. */
function renderContent(text: string): React.ReactNode[] {
  const parts = text.split(/(```[\s\S]*?```)/g);

  return parts.map((part, idx) => {
    if (part.startsWith('```') && part.endsWith('```')) {
      const code = part.slice(3, -3).replace(/^\w*\n/, ''); // strip lang hint
      return (
        <View key={idx} style={styles.codeBlock}>
          <Text style={styles.codeText}>{code.trim()}</Text>
        </View>
      );
    }
    return (
      <Text key={idx} style={styles.messageText}>
        {part}
      </Text>
    );
  });
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  const h = d.getHours().toString().padStart(2, '0');
  const m = d.getMinutes().toString().padStart(2, '0');
  return `${h}:${m}`;
}

export const ChatBubble: React.FC<ChatBubbleProps> = ({ message }) => {
  const isUser = message.role === 'user';

  return (
    <View
      style={[
        styles.row,
        isUser ? styles.rowUser : styles.rowAgent,
      ]}
    >
      <View
        style={[
          styles.bubble,
          isUser ? styles.bubbleUser : styles.bubbleAgent,
        ]}
      >
        {renderContent(message.text)}

        <View style={styles.meta}>
          <Text style={styles.timestamp}>{formatTime(message.timestamp)}</Text>
          {message.model && (
            <Text style={styles.model}>{message.model}</Text>
          )}
          {message.cost !== undefined && message.cost > 0 && (
            <Text style={styles.cost}>${message.cost.toFixed(4)}</Text>
          )}
        </View>
      </View>
    </View>
  );
};

const styles = StyleSheet.create({
  row: {
    paddingHorizontal: spacing.base,
    paddingVertical: spacing.xs,
  },
  rowUser: {
    alignItems: 'flex-end',
  },
  rowAgent: {
    alignItems: 'flex-start',
  },

  bubble: {
    maxWidth: '80%',
    borderRadius: radii.lg,
    padding: spacing.md,
  },
  bubbleUser: {
    backgroundColor: colors.accent,
    borderBottomRightRadius: radii.sm,
  },
  bubbleAgent: {
    backgroundColor: colors.bgElevated,
    borderBottomLeftRadius: radii.sm,
  },

  messageText: {
    color: colors.text,
    fontSize: typography.sizes.base,
    lineHeight: typography.sizes.base * typography.lineHeights.normal,
  },

  codeBlock: {
    backgroundColor: colors.bg,
    borderRadius: radii.sm,
    padding: spacing.sm,
    marginVertical: spacing.xs,
  },
  codeText: {
    fontFamily: 'monospace',
    color: colors.accentLight,
    fontSize: typography.sizes.sm,
    lineHeight: typography.sizes.sm * typography.lineHeights.relaxed,
  },

  meta: {
    flexDirection: 'row',
    gap: spacing.sm,
    marginTop: spacing.xs,
    alignItems: 'center',
  },
  timestamp: {
    color: colors.textMuted,
    fontSize: typography.sizes.xs,
  },
  model: {
    color: colors.textMuted,
    fontSize: typography.sizes.xs,
  },
  cost: {
    color: colors.textMuted,
    fontSize: typography.sizes.xs,
  },
});
