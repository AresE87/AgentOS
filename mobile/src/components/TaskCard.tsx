// ---------------------------------------------------------------------------
// AgentOS Mobile -- Task list item card
// ---------------------------------------------------------------------------

import React from 'react';
import { StyleSheet, Text, TouchableOpacity, View } from 'react-native';
import type { TaskResult } from '../types/api';
import { colors, radii, spacing, typography } from '../theme';

interface TaskCardProps {
  task: TaskResult;
  onPress?: (task: TaskResult) => void;
}

const STATUS_ICONS: Record<TaskResult['status'], string> = {
  completed: '\u2705',
  failed: '\u274C',
  running: '\u23F3',
  pending: '\u23F3',
};

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

export const TaskCard: React.FC<TaskCardProps> = ({ task, onPress }) => {
  return (
    <TouchableOpacity
      style={styles.card}
      activeOpacity={0.7}
      onPress={() => onPress?.(task)}
      disabled={!onPress}
    >
      <View style={styles.header}>
        <Text style={styles.statusIcon}>{STATUS_ICONS[task.status]}</Text>
        <Text style={styles.inputText} numberOfLines={2}>
          {task.output || '(no output)'}
        </Text>
      </View>

      <View style={styles.footer}>
        {task.model && (
          <View style={styles.badge}>
            <Text style={styles.badgeText}>{task.model}</Text>
          </View>
        )}

        <Text style={styles.detail}>${task.cost.toFixed(4)}</Text>
        <Text style={styles.detail}>{formatDuration(task.duration_ms)}</Text>
      </View>
    </TouchableOpacity>
  );
};

const styles = StyleSheet.create({
  card: {
    backgroundColor: colors.bgSecondary,
    borderRadius: radii.md,
    borderWidth: 1,
    borderColor: colors.border,
    padding: spacing.md,
    marginHorizontal: spacing.base,
    marginVertical: spacing.xs,
  },

  header: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    gap: spacing.sm,
  },
  statusIcon: {
    fontSize: typography.sizes.md,
    marginTop: 2,
  },
  inputText: {
    flex: 1,
    color: colors.text,
    fontSize: typography.sizes.base,
    lineHeight: typography.sizes.base * typography.lineHeights.normal,
  },

  footer: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: spacing.sm,
    marginTop: spacing.sm,
    paddingLeft: spacing.xl + spacing.sm, // align with text
  },

  badge: {
    backgroundColor: colors.accentMuted,
    borderRadius: radii.sm,
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.xxs,
  },
  badgeText: {
    color: colors.accentLight,
    fontSize: typography.sizes.xs,
    fontWeight: typography.weights.medium,
  },

  detail: {
    color: colors.textMuted,
    fontSize: typography.sizes.xs,
  },
});
