import React, { useState, useRef } from 'react';
import {
  View,
  Text,
  TextInput,
  TouchableOpacity,
  FlatList,
  StyleSheet,
  KeyboardAvoidingView,
  Platform,
  ActivityIndicator,
} from 'react-native';
import { getClient, sendMessage } from '../api/client';

interface Message {
  id: string;
  role: 'user' | 'agent';
  text: string;
}

export default function ChatScreen() {
  const [messages, setMessages] = useState<Message[]>([
    { id: '0', role: 'agent', text: 'AgentOS connected. How can I help?' },
  ]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const listRef = useRef<FlatList>(null);

  const send = async () => {
    if (!input.trim() || loading) return;
    const userMsg: Message = { id: Date.now().toString(), role: 'user', text: input };
    setMessages(prev => [...prev, userMsg]);
    const text = input;
    setInput('');
    setLoading(true);
    try {
      const client = await getClient();
      const taskId = await sendMessage(client, text);
      const agentMsg: Message = {
        id: (Date.now() + 1).toString(),
        role: 'agent',
        text: `Task queued: ${taskId}`,
      };
      setMessages(prev => [...prev, agentMsg]);
    } catch (e: any) {
      setMessages(prev => [
        ...prev,
        { id: (Date.now() + 1).toString(), role: 'agent', text: `Error: ${e.message}` },
      ]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : undefined}
    >
      <FlatList
        ref={listRef}
        data={messages}
        keyExtractor={m => m.id}
        onContentSizeChange={() => listRef.current?.scrollToEnd()}
        renderItem={({ item }) => (
          <View
            style={[
              styles.bubble,
              item.role === 'user' ? styles.userBubble : styles.agentBubble,
            ]}
          >
            <Text
              style={[
                styles.bubbleText,
                item.role === 'user' ? styles.userText : styles.agentText,
              ]}
            >
              {item.text}
            </Text>
          </View>
        )}
      />
      {loading && <ActivityIndicator color="#00ffff" style={{ margin: 8 }} />}
      <View style={styles.inputRow}>
        <TextInput
          style={styles.input}
          value={input}
          onChangeText={setInput}
          placeholder="Send a task..."
          placeholderTextColor="#555"
          onSubmitEditing={send}
        />
        <TouchableOpacity style={styles.sendBtn} onPress={send}>
          <Text style={styles.sendBtnText}>→</Text>
        </TouchableOpacity>
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: '#0a0a0f' },
  bubble: { maxWidth: '80%', borderRadius: 12, padding: 12, margin: 8 },
  userBubble: {
    alignSelf: 'flex-end',
    backgroundColor: '#00ffff22',
    borderWidth: 1,
    borderColor: '#00ffff44',
  },
  agentBubble: { alignSelf: 'flex-start', backgroundColor: '#1a1a2e' },
  bubbleText: { fontSize: 14 },
  userText: { color: '#00ffff' },
  agentText: { color: '#e0e0e0' },
  inputRow: {
    flexDirection: 'row',
    padding: 12,
    borderTopWidth: 1,
    borderTopColor: '#1a1a2e',
  },
  input: {
    flex: 1,
    backgroundColor: '#1a1a2e',
    color: '#fff',
    borderRadius: 8,
    paddingHorizontal: 12,
    fontSize: 14,
  },
  sendBtn: {
    backgroundColor: '#00ffff',
    borderRadius: 8,
    width: 44,
    height: 44,
    justifyContent: 'center',
    alignItems: 'center',
    marginLeft: 8,
  },
  sendBtnText: { color: '#0a0a0f', fontSize: 20, fontWeight: 'bold' },
});
