// ---------------------------------------------------------------------------
// AgentOS Mobile -- Playbooks screen
// ---------------------------------------------------------------------------

import React, { useCallback, useEffect, useState } from 'react';
import {
  ActivityIndicator,
  FlatList,
  StyleSheet,
  Switch,
  Text,
  View,
} from 'react-native';
import type { AgentOSClient } from '../api/client';
import type { Playbook } from '../types/api';
import { colors, radii, spacing, typography } from '../theme';

interface PlaybooksScreenProps {
  client: AgentOSClient | null;
}

const TIER_LABELS: Record<number, string> = {
  1: 'Basic',
  2: 'Standard',
  3: 'Advanced',
  4: 'Expert',
};

const TIER_COLORS: Record<number, string> = {
  1: colors.info,
  2: colors.success,
  3: colors.warning,
  4: colors.accent,
};

export const PlaybooksScreen: React.FC<PlaybooksScreenProps> = ({ client }) => {
  const [playbooks, setPlaybooks] = useState<Playbook[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!client) return;
    setIsLoading(true);
    setError(null);
    try {
      const res = await client.getPlaybooks();
      setPlaybooks(res.playbooks);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to load');
    } finally {
      setIsLoading(false);
    }
  }, [client]);

  useEffect(() => {
    load();
  }, [load]);

  const handleToggle = useCallback((_pb: Playbook) => {
    // Activate / deactivate will be wired up in a future iteration.
  }, []);

  const renderItem = useCallback(
    ({ item }: { item: Playbook }) => {
      const tierColor = TIER_COLORS[item.tier] ?? colors.textMuted;
      const tierLabel = TIER_LABELS[item.tier] ?? `Tier ${item.tier}`;

      return (
        <View style={styles.card}>
          <View style={styles.cardHeader}>
            <Text style={styles.name}>{item.name}</Text>
            <Switch
              value={item.active ?? false}
              onValueChange={() => handleToggle(item)}
              trackColor={{ false: colors.bgTertiary, true: colors.accentMuted }}
              thumbColor={item.active ? colors.accent : colors.textMuted}
            />
          </View>

          <View style={styles.badges}>
            <View style={[styles.tierBadge, { backgroundColor: `${tierColor}20` }]}>
              <Text style={[styles.tierText, { color: tierColor }]}>
                {tierLabel}
              </Text>
            </View>

            {item.permissions.map(perm => (
              <View key={perm} style={styles.permBadge}>
                <Text style={styles.permText}>{perm}</Text>
              </View>
            ))}
          </View>
        </View>
      );
    },
    [handleToggle],
  );

  if (isLoading && playbooks.length === 0) {
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

      {playbooks.length === 0 && !isLoading ? (
        <View style={styles.center}>
          <Text style={styles.emptyIcon}>{'\uD83D\uDCD6'}</Text>
          <Text style={styles.emptyTitle}>No playbooks installed</Text>
          <Text style={styles.emptySubtitle}>
            Install playbooks from the marketplace
          </Text>
        </View>
      ) : (
        <FlatList
          data={playbooks}
          renderItem={renderItem}
          keyExtractor={pb => pb.path}
          contentContainerStyle={styles.list}
          onRefresh={load}
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
    padding: spacing.base,
    gap: spacing.sm,
  },

  card: {
    backgroundColor: colors.bgSecondary,
    borderRadius: radii.md,
    borderWidth: 1,
    borderColor: colors.border,
    padding: spacing.md,
  },
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: spacing.sm,
  },
  name: {
    color: colors.text,
    fontSize: typography.sizes.md,
    fontWeight: typography.weights.semibold,
    flex: 1,
    marginRight: spacing.sm,
  },

  badges: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: spacing.xs,
  },
  tierBadge: {
    borderRadius: radii.sm,
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.xxs,
  },
  tierText: {
    fontSize: typography.sizes.xs,
    fontWeight: typography.weights.semibold,
  },
  permBadge: {
    backgroundColor: colors.bgTertiary,
    borderRadius: radii.sm,
    paddingHorizontal: spacing.sm,
    paddingVertical: spacing.xxs,
  },
  permText: {
    color: colors.textSecondary,
    fontSize: typography.sizes.xs,
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
