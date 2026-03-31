import React, { useEffect, useState } from 'react';
import {
  View,
  Text,
  TouchableOpacity,
  StyleSheet,
  ActivityIndicator,
  ScrollView,
  Alert,
} from 'react-native';
import { getClient, getAgentStatus, sendMessage, waitForTaskResult } from '../api/client';

interface AgentStatus {
  online: boolean;
  tasks_queued: number;
  api_version?: string;
  version?: string;
}

const QUICK_ACTIONS = [
  { label: 'Open Calculator', command: 'open calculator' },
  { label: 'Check Disk Space', command: 'check disk space' },
  { label: 'System Status', command: 'system status' },
];

export default function StatusScreen() {
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const fetchStatus = async () => {
    setLoading(true);
    try {
      const client = await getClient();
      const data = await getAgentStatus(client);
      setStatus({
        online: true,
        tasks_queued: data.tasks_queued ?? 0,
        api_version: data.api_version,
        version: data.version,
      });
    } catch {
      setStatus({ online: false, tasks_queued: 0 });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchStatus();
  }, []);

  const runQuickAction = async (action: { label: string; command: string }) => {
    setActionLoading(action.label);
    try {
      const client = await getClient();
      const queued = await sendMessage(client, action.command);
      const task = await waitForTaskResult(client, queued.task_id, 15, 1000);
      Alert.alert(
        action.label,
        task.result && task.result.length > 0
          ? task.result
          : `Task finished with status: ${task.status}`,
      );
      fetchStatus();
    } catch (e: any) {
      Alert.alert('Error', e.message);
    } finally {
      setActionLoading(null);
    }
  };

  return (
    <ScrollView style={styles.container} contentContainerStyle={styles.content}>
      <Text style={styles.sectionTitle}>Agent Status</Text>

      {loading ? (
        <ActivityIndicator color="#00ffff" style={{ marginVertical: 24 }} />
      ) : (
        <View style={styles.statusCard}>
          <View style={styles.statusRow}>
            <View style={[styles.dot, status?.online ? styles.dotOnline : styles.dotOffline]} />
            <Text style={styles.statusText}>{status?.online ? 'Online' : 'Offline'}</Text>
            <TouchableOpacity onPress={fetchStatus} style={styles.refreshBtn}>
              <Text style={styles.refreshBtnText}>Refresh</Text>
            </TouchableOpacity>
          </View>

          <View style={styles.divider} />

          <View style={styles.statRow}>
            <Text style={styles.statLabel}>Queued Tasks</Text>
            <Text style={styles.statValue}>{status?.tasks_queued ?? '-'}</Text>
          </View>

          {status?.api_version && (
            <View style={styles.statRow}>
              <Text style={styles.statLabel}>API Version</Text>
              <Text style={styles.statValue}>{status.api_version}</Text>
            </View>
          )}

          {status?.version && (
            <View style={styles.statRow}>
              <Text style={styles.statLabel}>Version</Text>
              <Text style={styles.statValue}>{status.version}</Text>
            </View>
          )}
        </View>
      )}

      <Text style={styles.sectionTitle}>Quick Actions</Text>

      {QUICK_ACTIONS.map(action => (
        <TouchableOpacity
          key={action.label}
          style={styles.actionBtn}
          onPress={() => runQuickAction(action)}
          disabled={actionLoading !== null}
        >
          {actionLoading === action.label ? (
            <ActivityIndicator color="#00ffff" />
          ) : (
            <Text style={styles.actionBtnText}>{action.label}</Text>
          )}
        </TouchableOpacity>
      ))}
    </ScrollView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: '#0a0a0f' },
  content: { padding: 24 },
  sectionTitle: {
    color: '#00ffff',
    fontSize: 16,
    fontWeight: 'bold',
    marginTop: 24,
    marginBottom: 12,
    textTransform: 'uppercase',
    letterSpacing: 1,
  },
  statusCard: {
    backgroundColor: '#1a1a2e',
    borderRadius: 12,
    padding: 16,
  },
  statusRow: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  dot: {
    width: 10,
    height: 10,
    borderRadius: 5,
    marginRight: 8,
  },
  dotOnline: { backgroundColor: '#00ff88' },
  dotOffline: { backgroundColor: '#ff4444' },
  statusText: { color: '#fff', fontSize: 16, fontWeight: '600', flex: 1 },
  refreshBtn: {
    borderWidth: 1,
    borderColor: '#00ffff44',
    borderRadius: 6,
    paddingHorizontal: 10,
    paddingVertical: 4,
  },
  refreshBtnText: { color: '#00ffff', fontSize: 12 },
  divider: { height: 1, backgroundColor: '#ffffff11', marginVertical: 12 },
  statRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: 8,
  },
  statLabel: { color: '#888', fontSize: 14 },
  statValue: { color: '#e0e0e0', fontSize: 14, fontWeight: '600' },
  actionBtn: {
    backgroundColor: '#1a1a2e',
    borderRadius: 10,
    padding: 16,
    marginBottom: 10,
    borderWidth: 1,
    borderColor: '#00ffff22',
    alignItems: 'center',
  },
  actionBtnText: { color: '#00ffff', fontSize: 15 },
});
