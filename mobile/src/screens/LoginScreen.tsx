// ---------------------------------------------------------------------------
// AgentOS Mobile -- Login / connection screen
// ---------------------------------------------------------------------------

import React, { useCallback, useState } from 'react';
import {
  ActivityIndicator,
  KeyboardAvoidingView,
  Platform,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import type { ConnectionConfig } from '../types/api';
import { colors, radii, spacing, typography } from '../theme';

interface LoginScreenProps {
  isConnecting: boolean;
  error: string | null;
  onConnect: (config: ConnectionConfig) => Promise<boolean>;
}

export const LoginScreen: React.FC<LoginScreenProps> = ({
  isConnecting,
  error,
  onConnect,
}) => {
  const [apiUrl, setApiUrl] = useState('');
  const [apiKey, setApiKey] = useState('');

  const canSubmit = apiUrl.trim().length > 0 && apiKey.trim().length > 0;

  const handleConnect = useCallback(async () => {
    if (!canSubmit) return;
    await onConnect({
      api_url: apiUrl.trim(),
      api_key: apiKey.trim(),
      display_name: apiUrl.trim(),
    });
  }, [apiUrl, apiKey, canSubmit, onConnect]);

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <ScrollView
        contentContainerStyle={styles.scroll}
        keyboardShouldPersistTaps="handled"
      >
        {/* Branding */}
        <View style={styles.brand}>
          <Text style={styles.logo}>{'>'}_</Text>
          <Text style={styles.title}>AgentOS</Text>
          <Text style={styles.subtitle}>Connect to your agent</Text>
        </View>

        {/* Form */}
        <View style={styles.form}>
          <Text style={styles.label}>API URL</Text>
          <TextInput
            style={styles.input}
            placeholder="https://192.168.1.100:8000"
            placeholderTextColor={colors.textMuted}
            value={apiUrl}
            onChangeText={setApiUrl}
            autoCapitalize="none"
            autoCorrect={false}
            keyboardType="url"
            returnKeyType="next"
            editable={!isConnecting}
          />

          <Text style={styles.label}>API Key</Text>
          <TextInput
            style={styles.input}
            placeholder="Your API key"
            placeholderTextColor={colors.textMuted}
            value={apiKey}
            onChangeText={setApiKey}
            secureTextEntry
            autoCapitalize="none"
            autoCorrect={false}
            returnKeyType="done"
            onSubmitEditing={handleConnect}
            editable={!isConnecting}
          />

          {error && (
            <View style={styles.errorBox}>
              <Text style={styles.errorText}>{error}</Text>
            </View>
          )}

          <TouchableOpacity
            style={[
              styles.connectBtn,
              (!canSubmit || isConnecting) && styles.connectBtnDisabled,
            ]}
            onPress={handleConnect}
            activeOpacity={0.7}
            disabled={!canSubmit || isConnecting}
          >
            {isConnecting ? (
              <ActivityIndicator color={colors.text} />
            ) : (
              <Text style={styles.connectBtnText}>Connect</Text>
            )}
          </TouchableOpacity>

          <View style={styles.dividerRow}>
            <View style={styles.dividerLine} />
            <Text style={styles.dividerText}>or</Text>
            <View style={styles.dividerLine} />
          </View>

          <TouchableOpacity style={styles.qrBtn} activeOpacity={0.7}>
            <Text style={styles.qrBtnText}>Scan QR Code</Text>
          </TouchableOpacity>
          <Text style={styles.qrHint}>
            Scan the QR code shown in the AgentOS desktop dashboard
          </Text>
        </View>
      </ScrollView>
    </KeyboardAvoidingView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.bg,
  },
  scroll: {
    flexGrow: 1,
    justifyContent: 'center',
    padding: spacing['2xl'],
  },

  // Branding
  brand: {
    alignItems: 'center',
    marginBottom: spacing['3xl'],
  },
  logo: {
    color: colors.accent,
    fontSize: typography.sizes['3xl'],
    fontWeight: typography.weights.bold,
    fontFamily: 'monospace',
    marginBottom: spacing.sm,
  },
  title: {
    color: colors.text,
    fontSize: typography.sizes['2xl'],
    fontWeight: typography.weights.bold,
    marginBottom: spacing.xs,
  },
  subtitle: {
    color: colors.textSecondary,
    fontSize: typography.sizes.base,
  },

  // Form
  form: {},
  label: {
    color: colors.textSecondary,
    fontSize: typography.sizes.sm,
    fontWeight: typography.weights.medium,
    marginBottom: spacing.xs,
    marginTop: spacing.md,
  },
  input: {
    backgroundColor: colors.bgSecondary,
    borderWidth: 1,
    borderColor: colors.border,
    borderRadius: radii.md,
    paddingHorizontal: spacing.md,
    paddingVertical: spacing.md,
    color: colors.text,
    fontSize: typography.sizes.base,
  },

  errorBox: {
    backgroundColor: `${colors.error}20`,
    borderRadius: radii.sm,
    padding: spacing.md,
    marginTop: spacing.md,
  },
  errorText: {
    color: colors.error,
    fontSize: typography.sizes.sm,
    textAlign: 'center',
  },

  connectBtn: {
    backgroundColor: colors.accent,
    borderRadius: radii.md,
    paddingVertical: spacing.md,
    alignItems: 'center',
    marginTop: spacing.lg,
  },
  connectBtnDisabled: {
    opacity: 0.5,
  },
  connectBtnText: {
    color: colors.text,
    fontSize: typography.sizes.md,
    fontWeight: typography.weights.semibold,
  },

  // Divider
  dividerRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginVertical: spacing.lg,
  },
  dividerLine: {
    flex: 1,
    height: 1,
    backgroundColor: colors.border,
  },
  dividerText: {
    color: colors.textMuted,
    fontSize: typography.sizes.sm,
    marginHorizontal: spacing.md,
  },

  // QR button
  qrBtn: {
    backgroundColor: colors.bgSecondary,
    borderWidth: 1,
    borderColor: colors.border,
    borderRadius: radii.md,
    paddingVertical: spacing.md,
    alignItems: 'center',
  },
  qrBtnText: {
    color: colors.text,
    fontSize: typography.sizes.base,
    fontWeight: typography.weights.medium,
  },
  qrHint: {
    color: colors.textMuted,
    fontSize: typography.sizes.xs,
    textAlign: 'center',
    marginTop: spacing.sm,
  },
});
