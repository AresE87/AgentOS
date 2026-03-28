// ---------------------------------------------------------------------------
// AgentOS Mobile -- Root application component
// ---------------------------------------------------------------------------

import React from 'react';
import { StatusBar, StyleSheet } from 'react-native';
import { NavigationContainer, DefaultTheme } from '@react-navigation/native';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { SafeAreaProvider } from 'react-native-safe-area-context';

import { useAgent, useTask, useTasks, useStatus } from './hooks/useAgent';
import { ChatScreen } from './screens/ChatScreen';
import { TasksScreen } from './screens/TasksScreen';
import { PlaybooksScreen } from './screens/PlaybooksScreen';
import { SettingsScreen } from './screens/SettingsScreen';
import { LoginScreen } from './screens/LoginScreen';
import { colors } from './theme';

// ---------------------------------------------------------------------------
// Navigation setup
// ---------------------------------------------------------------------------

type TabParamList = {
  Chat: undefined;
  Tasks: undefined;
  Playbooks: undefined;
  Settings: undefined;
};

const Tab = createBottomTabNavigator<TabParamList>();

const navTheme = {
  ...DefaultTheme,
  dark: true,
  colors: {
    ...DefaultTheme.colors,
    primary: colors.accent,
    background: colors.bg,
    card: colors.bgSecondary,
    text: colors.text,
    border: colors.border,
    notification: colors.accent,
  },
};

// Simple text-based tab icons (avoids icon library dependency)
const TAB_ICONS: Record<keyof TabParamList, string> = {
  Chat: '\uD83D\uDCAC',
  Tasks: '\uD83D\uDCCB',
  Playbooks: '\uD83D\uDCD6',
  Settings: '\u2699\uFE0F',
};

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

const App: React.FC = () => {
  const agent = useAgent();
  const taskHook = useTask(agent.client);
  const tasksHook = useTasks(agent.client);
  const statusHook = useStatus(agent.client);

  // Gate: show login when not connected
  if (!agent.isConnected) {
    return (
      <SafeAreaProvider>
        <StatusBar barStyle="light-content" backgroundColor={colors.bg} />
        <LoginScreen
          isConnecting={agent.isConnecting}
          error={agent.error}
          onConnect={agent.connect}
        />
      </SafeAreaProvider>
    );
  }

  return (
    <SafeAreaProvider>
      <StatusBar barStyle="light-content" backgroundColor={colors.bg} />
      <NavigationContainer theme={navTheme}>
        <Tab.Navigator
          screenOptions={({ route }) => ({
            headerStyle: styles.header,
            headerTitleStyle: styles.headerTitle,
            headerTintColor: colors.text,
            tabBarStyle: styles.tabBar,
            tabBarActiveTintColor: colors.accent,
            tabBarInactiveTintColor: colors.textMuted,
            tabBarIcon: ({ focused }) => {
              const icon = TAB_ICONS[route.name];
              return (
                <React.Fragment>
                  {/* Render emoji icon with opacity change on focus */}
                  {React.createElement(
                    require('react-native').Text,
                    {
                      style: {
                        fontSize: 20,
                        opacity: focused ? 1 : 0.5,
                      },
                    },
                    icon,
                  )}
                </React.Fragment>
              );
            },
          })}
        >
          <Tab.Screen name="Chat">
            {() => <ChatScreen client={agent.client} />}
          </Tab.Screen>

          <Tab.Screen name="Tasks">
            {() => <TasksScreen tasksHook={tasksHook} />}
          </Tab.Screen>

          <Tab.Screen name="Playbooks">
            {() => <PlaybooksScreen client={agent.client} />}
          </Tab.Screen>

          <Tab.Screen name="Settings">
            {() => (
              <SettingsScreen
                client={agent.client}
                config={agent.config}
                onDisconnect={agent.disconnect}
              />
            )}
          </Tab.Screen>
        </Tab.Navigator>
      </NavigationContainer>
    </SafeAreaProvider>
  );
};

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const styles = StyleSheet.create({
  header: {
    backgroundColor: colors.bgSecondary,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
    elevation: 0,
    shadowOpacity: 0,
  },
  headerTitle: {
    color: colors.text,
    fontWeight: '600',
    fontSize: 17,
  },
  tabBar: {
    backgroundColor: colors.bgSecondary,
    borderTopWidth: 1,
    borderTopColor: colors.border,
    paddingBottom: 4,
    height: 56,
  },
});

export default App;
