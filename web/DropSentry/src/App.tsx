import { AppShell, Divider, Group, rem, Stack, Title, NavLink } from '@mantine/core';
import '@mantine/core/styles.css';
import { Route, Routes, useLocation, useNavigate } from 'react-router-dom';
import Dashboard from './pages/Dashboard';
import Settings from './pages/Settings';
import Perfomance from './pages/Perfomance';

function App() {
  const navigate = useNavigate();
  const location = useLocation();
  return(
    <AppShell navbar={{ width: 250, breakpoint: "sm" }} padding="md" header={{ height: 36 }}>

      <AppShell.Navbar p="md" styles={{ navbar: {
        height: rem(800),
        maxHeight: rem(400),
        top: 'auto',
        bottom: 'auto',
        left: rem(10),
        borderRadius: rem(16),
        overflow: 'hidden',
        boxShadow: '0 10px 40px rgba(0, 0, 0, 0.4)',
        border: '1px solid var(--mantine-color-dark-5)',
        transition: 'all 0.3s ease',
      }}}>

        <Stack gap="sm">
          <Group>
            <Title order={4}>DROPSENTRY</Title>
          </Group>

          <Divider my = "md" label="main" labelPosition="left"></Divider>

          <NavLink
            label="Dashboard"
            active={location.pathname === "/"}
            onClick={() => navigate("/")}
            styles={ (theme) => ({
              root: {
                borderRadius: theme.radius.md,
              }
            })}
          />

          <NavLink 
            label="Perfomance"
            active={location.pathname === "/perfomance"}
            onClick={() => navigate("/perfomance")}
            styles={ (theme) => ({
              root: {
                borderRadius: theme.radius.md,
              }
            })}
          />         

          <Divider my="md" label="settings" labelPosition="left"></Divider>

          <NavLink
            label="Settings"
            active={location.pathname === "/settings"}
            onClick={() => navigate("/settings")}
            styles={ (theme) => ({
              root: {
                borderRadius: theme.radius.md,
              }
            })}
          />
        </Stack>
      </AppShell.Navbar>
      <AppShell.Main>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/perfomance" element={<Perfomance />} />
        </Routes>
      </AppShell.Main>
    </AppShell>
  )
}

export default App
