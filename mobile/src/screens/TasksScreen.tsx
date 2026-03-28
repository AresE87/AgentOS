// ---------------------------------------------------------------------------
// AgentOS Mobile -- Tasks history screen
// ---------------------------------------------------------------------------

import React, { useCallback } from 'react';
import {
  ActivityIndicator,
  FlatList,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { TaskCard } from '../components/TaskCard';
import type { UseTasksReturn } from '../hooks/useAgent';
import type { TaskResult } from '../types/api';
import { colors, spacing, typography } from '../theme';

interface TasksScreenProps {
  tasksHook: UseTasksReturn;
}

export const TasksScreen: React.FC<TasksScreenProps> = ({ tasksHook }) => {
  const { tasks, isLoading, error, refresh } = tasksHook;

  const handlePress = useCallback((_task: TaskResult) => {
    // Navigate to task detail in a future iteration.
  }, []);

  const renderItem = useCallback(
    ({ item }: { item: TaskResult }) => (
      <TaskCard task={item} onPress={handlePress} />
    ),
    [handlePress],
  );

  if (isLoading && tasks.length === 0) {
    return (
      <View style={styles.center}>
        <ActivityIndicator size="large" color={colors.accent} />
      </View>
    );
  }

  return (
    <View style={styles.container}>
      {error && (
        <View style={styles.errorBanner}>
          <Text style={styles.errorText}>{error}</Text>
        </View>
      )}

      {tasks.length === 0 ? (
        <View style={styles.center}>
          <Text style={styles.emptyIcon}>{'\uD83D\uDCCB'}</Text>
          <Text style={styles.emptyTitle}>No tasks yet</Text>
          <Text style={styles.emptySubtitle}>
            Tasks you run will appear here
          </Text>
        </View>
      ) : (
        <FlatList
          data={tasks}
          renderItem={renderItem}
          keyExtractor={t => t.task_id}
          contentContainerStyle={styles.list}
          onRefresh={refresh}
          refreshing={isLoading}
        />
      )}
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.bg,
  },
  center: {
    flex: 1,
    backgroundColor: colors.bg,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: spacing['2xl'],
  },
  list: {
    paddingVertical: spacing.sm,
  },

  emptyIcon: {
    fontSize: 48,
    marginBottom: spacing.md,
  },
  emptyTitle: {
    color: colors.text,
    fontSize: typography.sizes.lg,
    fontWeight: typography.weights.semibold,
    marginBottom: spacing.xs,
  },
  emptySubtitle: {
    color: colors.textSecondary,
    fontSize: typography.sizes.base,
    textAlign: 'center',
  },

  errorBanner: {
    backgroundColor: colors.error,
    paddingHorizontal: spacing.base,
    paddingVertical: spacing.sm,
  },
  errorText: {
    color: colors.text,
    fontSize: typography.sizes.sm,
    textAlign: 'center',
  },
});
