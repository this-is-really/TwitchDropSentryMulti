import { Group, Paper, RingProgress, Stack, Text, Title } from "@mantine/core";
import { useEffect, useState } from "react";

interface SystemInfo {
  total_memory: number;
  process_memory: number;
  cpu_name: string;
  cpu_usage: number;
}

export default function Performance() {
    const [info, setInfo] = useState<SystemInfo | null>(null);

    useEffect(() => {
        const fetchPerformance = async () => {
          try {
            const response = await fetch('/api/performance');
            if (!response.ok) throw new Error('Network response was not ok');

            const data: SystemInfo = await response.json();
            setInfo(data);
          } catch (err) {
            console.error("Fetch error:", err);
          }
        };

        fetchPerformance();

        const intervalId = setInterval(fetchPerformance, 15000);

        return () => clearInterval(intervalId);
    }, []);

    if (!info) {
        return <Text>Loading...</Text>;
    }

    const ramUsedMB = (info.process_memory / 1024 / 1024).toFixed(1);
    const ramTotalGB = (info.total_memory / 1024 / 1024 / 1024).toFixed(1);
    const ramPercent = Math.min(
        100,
        Math.round((info.process_memory / info.total_memory) * 100)
    );

    const cpuPercent = Math.round(info.cpu_usage);

    return(
        <Stack>
            <Title order={2}>System Status</Title>
            <Group grow align="center" justify="center" wrap="nowrap">
                <Paper withBorder p="md" radius="md" shadow="sm" style={{ maxWidth: 280 }}>
                  <Stack align="center" gap="xs">
                    <RingProgress
                      size={160}
                      thickness={16}
                      roundCaps
                      sections={[{ value: ramPercent, color: ramPercent > 85 ? 'red' : 'blue' }]}
                      label={
                        <div style={{ textAlign: 'center' }}>
                          <Text fw={700} size="lg">
                            {ramPercent}%
                          </Text>
                          <Text size="xs" c="dimmed">
                            RAM
                          </Text>
                        </div>
                      }
                    />
                    <div style={{ textAlign: 'center' }}>
                      <Text fw={500}>
                        The application uses {ramUsedMB} MB
                      </Text>
                      <Text size="sm" c="dimmed">
                        from {ramTotalGB} GB total
                      </Text>
                    </div>
                  </Stack>
                </Paper>
                  
                <Paper withBorder p="md" radius="md" shadow="sm" style={{ maxWidth: 280 }}>
                  <Stack align="center" gap="xs">
                    <RingProgress
                      size={160}
                      thickness={16}
                      roundCaps
                      sections={[{ value: cpuPercent, color: cpuPercent > 80 ? 'orange' : 'teal' }]}
                      label={
                        <div style={{ textAlign: 'center' }}>
                          <Text fw={700} size="lg">
                            {cpuPercent}%
                          </Text>
                          <Text size="xs" c="dimmed">
                            CPU
                          </Text>
                        </div>
                      }
                    />
                    <div style={{ textAlign: 'center' }}>
                      <Text fw={500}>{info.cpu_name}</Text>
                      <Text size="sm" c="dimmed">
                        current loading
                      </Text>
                    </div>
                  </Stack>
                </Paper>
            </Group>
        </Stack>
    )
}