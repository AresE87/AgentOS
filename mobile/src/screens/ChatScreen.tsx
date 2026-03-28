// ---------------------------------------------------------------------------
// AgentOS Mobile -- Chat screen
// ---------------------------------------------------------------------------

import React, { useCallback, useRef, useState } from 'react';
import {
  ActivityIndicator,
  FlatList,
  KeyboardAvoidingView,
  Platform,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import { ChatBubble } from '../components/ChatBubble';
import type { ChatMessage } from '../types/api';
import type { AgentOSClient } from '../api/client';
import { colors, radii, spacing, typography } from '../theme';

interface ChatScreenProps {
  client: AgentOSClient | null;
}

let nextId = 1;
function genId(): string {
  return `msg_${Date.now()}_${nextId++}`;
}

export const ChatScreen: React.FC<ChatScreenProps> = ({ client }) => {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const listRef = useRef<FlatList<ChatMessage>>(null);

  const send = useCallback(async () => {
    const text = input.trim();
    if (!text || !client) return;

    const userMsg: ChatMessage = {
      id: genId(),
      role: 'user',
      text,
      timestamp: Date.now(),
    };

    setMessages(prev => [...prev, userMsg]);
    setInput('');
    setIsLoading(true);

    try {
      const result = await client.runTask(text);

      const agentMsg: ChatMessage = {
        id: genId(),
        role: 'agent',
        text: result.output || '(no output)',
        timestamp: Date.now(),
        taskId: result.task_id,
        status: result.status,
        cost: result.cost,
        model: result.model ?? undefined,
      };
      setMessages(prev => [...prev, agentMsg]);
    } catch (err: unknown) {
      const errText = err instanceof Error ? err.message : 'Unknown error';
      const errMsg: ChatMessage = {
        id: genId(),
        role: 'agent',
        text: `Error: ${errText}`,
        timestamp: Date.now(),
        status: 'failed',
      };
      setMessages(prev => [...prev, errMsg]);
    } finally {
      setIsLoading(false);
    }
  }, [input, client]);

  const handleRefresh = useCallback(() => {
    // Pull-to-refresh could reload recent task history in a future iteration.
    // For now it is a no-op placeholder.
  }, []);

  const renderItem = useCallback(
    ({ item }: { item: ChatMessage }) => <ChatBubble message={item} />,
    [],
  );

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
      keyboardVerticalOffset={90}
    >
      {messages.length === 0 ? (
        <View style={styles.empty}>
          <Text style={styles.emptyIcon}>{'>'}_</Text>
          <Text style={styles.emptyTitle}>AgentOS</Text>
          <Text style={styles.emptySubtitle}>
            Send a message to start a task
          </Text>
        </View>
      ) : (
        <FlatList
          ref={listRef}
          data={messages}
          renderItem={renderItem}
          keyExtractor={m => m.id}
          contentContainerStyle={styles.list}
          onContentSizeChange={() =>
            listRef.current?.scrollToEnd({ animated: true })
          }
          onRefresh={handleRefresh}
          refreshing={false}
        />
      )}

      {/* Input bar */}
      <View style={styles.inputBar}>
        <TextInput
          style={styles.textInput}
          placeholder="Ask the agent..."
          placeholderTextColor={colors.textMuted}
          value={input}
          onChangeText={setInput}
          onSubmitEditing={send}
          returnKeyType="send"
          editable={!isLoading}
          multiline
          maxLength={4000}
        />

        {isLoading ? (
          <ActivityIndicator color={colors.accent} style={styles.sendBtn} />
        ) : (
          <TouchableOpacity
            style={[
              styles.sendBtn,
              !input.trim() && styles.sendBtnDisabled,
            ]}
            onPress={send}
            disabled={!input.trim()}
          >
            <Text style={styles.sendIcon}>{'\u2191'}</Text>
          </TouchableOpacity>
        )}
      </View>
    </KeyboardAvoidingView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.bg,
  },

  list: {
    paddingVertical: spacing.sm,
  },

  // Empty state
  empty: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: spacing['2xl'],
  },
  emptyIcon: {
    color: colors.accent,
    fontSize: typography.sizes['2xl'],
    fontWeight: typography.weights.bold,
    fontFamily: 'monospace',
    marginBottom: spacing.md,
  },
  emptyTitle: {
    color: colors.text,
    fontSize: typography.sizes.xl,
    fontWeight: typography.weights.bold,
    marginBottom: spacing.xs,
  },
  emptySubtitle: {
    color: colors.textSecondary,
    fontSize: typography.sizes.base,
    textAlign: 'center',
  },

  // Input bar
  inputBar: {
    flexDirection: 'row',
    alignItems: 'flex-end',
    borderTopWidth: 1,
    borderTopColor: colors.border,
    backgroundColor: colors.bgSecondary,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    gap: spacing.sm,
  },
  textInput: {
    flex: 1,
    backgroundColor: colors.bgTertiary,
    borderRadius: radii.lg,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.sm,
    color: colors.text,
    fontSize: typography.sizes.base,
    maxHeight: 120,
  },
  sendBtn: {
    width: 40,
    height: 40,
    borderRadius: radii.full,
    backgroundColor: colors.accent,
    justifyContent: 'center',
    alignItems: 'center',
  },
  sendBtnDisabled: {
    backgroundColor: colors.bgTertiary,
  },
  sendIcon: {
    color: colors.text,
    fontSize: typography.sizes.md,
    fontWeight: typography.weights.bold,
  },
});
