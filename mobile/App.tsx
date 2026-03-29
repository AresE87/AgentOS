import React, { useEffect, useState } from 'react';
import { NavigationContainer } from '@react-navigation/native';
import { createStackNavigator } from '@react-navigation/stack';
import AsyncStorage from '@react-native-async-storage/async-storage';
import SetupScreen from './src/screens/SetupScreen';
import ChatScreen from './src/screens/ChatScreen';
import StatusScreen from './src/screens/StatusScreen';

const Stack = createStackNavigator();

export default function App() {
  const [initialRoute, setInitialRoute] = useState<string | null>(null);

  useEffect(() => {
    AsyncStorage.getItem('apiKey').then(key => {
      setInitialRoute(key ? 'Chat' : 'Setup');
    });
  }, []);

  if (!initialRoute) return null;

  return (
    <NavigationContainer>
      <Stack.Navigator
        initialRouteName={initialRoute}
        screenOptions={{
          headerStyle: { backgroundColor: '#0a0a0f' },
          headerTintColor: '#00ffff',
          headerTitleStyle: { color: '#fff' },
        }}
      >
        <Stack.Screen name="Setup" component={SetupScreen} options={{ title: 'Setup' }} />
        <Stack.Screen name="Chat" component={ChatScreen} options={{ title: 'AgentOS' }} />
        <Stack.Screen name="Status" component={StatusScreen} options={{ title: 'Status' }} />
      </Stack.Navigator>
    </NavigationContainer>
  );
}
