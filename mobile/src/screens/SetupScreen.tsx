import React, { useState } from 'react';
import { View, Text, TextInput, TouchableOpacity, StyleSheet, ActivityIndicator, Alert } from 'react-native';
import { saveClient, checkHealth } from '../api/client';

export default function SetupScreen({ navigation }: any) {
  const [host, setHost] = useState('http://192.168.1.100:8080');
  const [apiKey, setApiKey] = useState('');
  const [testing, setTesting] = useState(false);

  const testConnection = async () => {
    setTesting(true);
    const ok = await checkHealth(host);
    setTesting(false);
    Alert.alert(ok ? 'Connected' : 'Failed', ok ? 'AgentOS is reachable' : 'Cannot reach AgentOS. Check IP and port.');
  };

  const save = async () => {
    await saveClient(host, apiKey);
    navigation.replace('Chat');
  };

  return (
    <View style={styles.container}>
      <Text style={styles.title}>Connect to AgentOS</Text>
      <Text style={styles.label}>Desktop IP:Port</Text>
      <TextInput style={styles.input} value={host} onChangeText={setHost} autoCapitalize="none" />
      <Text style={styles.label}>API Key</Text>
      <TextInput style={styles.input} value={apiKey} onChangeText={setApiKey} autoCapitalize="none" secureTextEntry />
      <TouchableOpacity style={styles.testBtn} onPress={testConnection} disabled={testing}>
        {testing ? <ActivityIndicator color="#00ffff" /> : <Text style={styles.testBtnText}>Test Connection</Text>}
      </TouchableOpacity>
      <TouchableOpacity style={styles.saveBtn} onPress={save}>
        <Text style={styles.saveBtnText}>Save &amp; Continue</Text>
      </TouchableOpacity>
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: '#0a0a0f', padding: 24, justifyContent: 'center' },
  title: { color: '#00ffff', fontSize: 24, fontWeight: 'bold', marginBottom: 32 },
  label: { color: '#888', fontSize: 12, marginBottom: 4, marginTop: 16 },
  input: { backgroundColor: '#1a1a2e', color: '#fff', borderRadius: 8, padding: 12, fontSize: 14 },
  testBtn: { marginTop: 24, borderWidth: 1, borderColor: '#00ffff', borderRadius: 8, padding: 12, alignItems: 'center' },
  testBtnText: { color: '#00ffff', fontSize: 14 },
  saveBtn: { marginTop: 12, backgroundColor: '#00ffff', borderRadius: 8, padding: 14, alignItems: 'center' },
  saveBtnText: { color: '#0a0a0f', fontWeight: 'bold', fontSize: 16 },
});
