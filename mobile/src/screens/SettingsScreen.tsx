// ---------------------------------------------------------------------------
// AgentOS Mobile -- Settings screen
// ---------------------------------------------------------------------------

import React, { useCallback, useEffect, useState } from 'react';
import {
  Alert,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import { StatCard } from '../components/StatCard';
import type { AgentOSClient } from '../api/client';
import type { AgentStatus, ConnectionConfig } from '../types/api';
import { colors, radii, spacing, typography } from '../theme';

interface SettingsScreenProps {
  client: AgentOSClient | null;
  config: ConnectionConfig | null;
  onDisconnect: () => void;
}

const APP_VERSION = '0.1.0';

export const SettingsScreen: React.FC<SettingsScreenProps> = ({
  client,
  config,
  onDisconnect,
}) => {
  const [status, setStatus] = useState<AgentStatus | null>(null);

  useEffect(() => {
    if (!client) return;
    client.getStatus().then(setStatus).catch(() => {});
  }, [client]);

  const confirmDisconnect = useCallback(() => {
    Alert.alert(
      'Disconnect',
      'Are you sure you want to disconnect from this agent?',
      [
        { text: 'Cancel', style: 'cancel' },
        { text: 'Disconnect', style: 'destructive', onPress: onDisconnect },
      ],
    );
  }, [onDisconnect]);

  return (
    <ScrollView
      style={styles.container}
      contentContainerStyle={styles.content}
    >
      {/* Connection info */}
      <Text style={styles.sectionTitle}>Connection</Text>
      <View style={styles.card}>
        <Row label="API URL" value={config?.api_url ?? 'Not connected'} />
        <Row
          label="Status"
          value={status ? 'Connected' : 'Offline'}
          valueColor={status ? colors.success : colors.error}
        />
        <Row label="Agent State" value={status?.state ?? '-'} />
      </View>

      {/* Session stats */}
      {status && (
        <>
          <Text style={styles.sectionTitle}>Session</Text>
          <View style={styles.statsRow}>
            <StatCard
              label="Tasks"
              value={status.session_stats.tasks}
              accentColor={colors.info}
            />
            <StatCard
              label="Cost"
              value={`$${status.session_stats.cost.toFixed(4)}`}
              accentColor={colors.warning}
            />
            <StatCard
              label="Tokens"
              value={formatNumber(status.session_stats.tokens)}
              accentColor={colors.accent}
            />
          </View>
        </>
      )}

      {/* Providers */}
      {status && status.providers.length > 0 && (
        <>
          <Text style={styles.sectionTitle}>Providers</Text>
          <View style={styles.card}>
            {status.providers.map(p => (
              <Row key={p} label={p} value="Active" valueColor={colors.success} />
            ))}
          </View>
        </>
      )}

      {/* About */}
      <Text style={styles.sectionTitle}>About</Text>
      <View style={styles.card}>
        <Row label="App Version" value={APP_VERSION} />
        <Row label="Platform" value="React Native 0.73" />
      </View>

      {/* Disconnect */}
      <TouchableOpacity
        style={styles.disconnectBtn}
        onPress={confirmDisconnect}
        activeOpacity={0.7}
      >
        <Text style={styles.disconnectText}>Disconnect</Text>
      </TouchableOpacity>
    </ScrollView>
  );
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

interface RowProps {
  label: string;
  value: string;
  valueColor?: string;
}

const Row: React.FC<RowProps> = ({ label, value, valueColor }) => (
  <View style={styles.row}>
    <Text style={styles.rowLabel}>{label}</Text>
    <Text style={[styles.rowValue, valueColor ? { color: valueColor } : undefined]}>
      {value}
    </Text>
  </View>
);

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.bg,
  },
  content: {
    padding: spacing.base,
    paddingBottom: spacing['4xl'],
  },

  sectionTitle: {
    color: colors.textSecondary,
    fontSize: typography.sizes.xs,
    fontWeight: typography.weights.semibold,
    textTransform: 'uppercase',
    letterSpacing: 1,
    marginTop: spacing.lg,
    marginBottom: spacing.sm,
    paddingHorizontal: spacing.xs,
  },

  card: {
    backgroundColor: colors.bgSecondary,
    borderRadius: radii.md,
    borderWidth: 1,
    borderColor: colors.border,
    overflow: 'hidden',
  },

  row: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  rowLabel: {
    color: colors.textSecondary,
    fontSize: typography.sizes.base,
  },
  rowValue: {
    color: colors.text,
    fontSize: typography.sizes.base,
    fontWeight: typography.weights.medium,
    maxWidth: '60%',
    textAlign: 'right',
  },

  statsRow: {
    flexDirection: 'row',
    gap: spacing.sm,
  },

  disconnectBtn: {
    marginTop: spacing['2xl'],
    backgroundColor: colors.bgSecondary,
    borderRadius: radii.md,
    borderWidth: 1,
    borderColor: colors.error,
    paddingVertical: spacing.md,
    alignItems: 'center',
  },
  disconnectText: {
    color: colors.error,
    fontSize: typography.sizes.base,
    fontWeight: typography.weights.semibold,
  },
});
