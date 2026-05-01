import { DragDropContext, Draggable, Droppable, type DropResult } from "@hello-pangea/dnd";
import { ActionIcon, Button, Card, Group, List, Stack, Switch, Text, TextInput, Title } from "@mantine/core";
import { IconGripVertical, IconTrash } from "@tabler/icons-react";
import { useEffect, useState } from "react";

interface Game {
  name: string;
  position: number;
}

interface Proxy {
  url: string;
}

export default function Settings() {
  const [games, setGames] = useState<Game[]>([]);
  const [proxies, setProxies] = useState<Proxy[]>([]);
  const [newGame, setNewGame] = useState('');
  const [newProxy, setNewProxy] = useState('');
  const [autostart, setAutostart] = useState(false);

  useEffect(() => {
    fetch('/api/games')
      .then((res) => res.json())
      .then(setGames);

    fetch('/api/proxies')
      .then((res) => res.json())
      .then((data: string[]) => {
        const formatted = data.map(url => ({ url }));
        setProxies(formatted);
      });
  }, []);

  const addProxy = () => {
    if (!newProxy.trim()) return;
    fetch('/api/proxies', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: newProxy.trim() }),
    })
      .then((res) => res.json())
      .then((proxy) => {
        setProxies((prev) => [...prev, proxy]);
        setNewProxy('');
      });
  };

  const deleteProxy = (url: string) => {
    if (!url) return;

    fetch('/api/proxies', {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url }),
    })
      .then(() => {
        setProxies((prev) => prev.filter((p) => p.url !== url));
      });
  };

  const addGame = () => {
    if (!newGame.trim()) return;
    fetch('/api/games', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: newGame.trim() }),
    })
      .then((res) => res.json())
      .then((game) => {
        setGames((prev) => [...prev, game]);
        setNewGame('');
      });
  };

  const deleteGame = (position: number) => {
    fetch('/api/games', {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ position }),
    })
      .then(() => {
        setGames((prev) => prev.filter((game) => game.position !== position));
      });
  };

  const handleDragEnd = (result: DropResult) => {
    if (!result.destination) return;

    const items = Array.from(games);
    const [reorderedItem] = items.splice(result.source.index, 1);
    items.splice(result.destination.index, 0, reorderedItem);

    const updatedGames = items.map((game, index) => ({
      ...game,
      position: index,
    }));

    setGames(updatedGames);

    fetch('/api/games/reorder', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        game_name: reorderedItem.name,
        position: result.destination.index,
      }),
    });
  };

  return (
    <Stack gap="xl" p="md">
      <Title order={2}>Settings</Title>

      <Card withBorder shadow="sm" p="lg">
        <Title order={3} mb="md">Game</Title>
        
        <Group mb="md">
          <TextInput
            placeholder="Game name"
            value={newGame}
            onChange={(e) => setNewGame(e.currentTarget.value)}
            style={{ flex: 1 }}
          />
          <Button onClick={addGame}>Add</Button>
        </Group>

        <DragDropContext onDragEnd={handleDragEnd}>
          <Droppable droppableId="games">
            {(provided) => (
              <List
                {...provided.droppableProps}
                ref={provided.innerRef}
                spacing="xs"
              >
                {games
                  .sort((a, b) => a.position - b.position)
                  .map((game, index) => (
                    <Draggable
                      key={game.name}
                      draggableId={game.name}
                      index={index}
                    >
                      {(provided, snapshot) => (
                        <List.Item
                          ref={provided.innerRef}
                          {...provided.draggableProps}
                          style={{
                            ...provided.draggableProps.style,
                            backgroundColor: snapshot.isDragging
                              ? 'var(--mantine-color-gray-1)'
                              : undefined,
                            borderRadius: 'var(--mantine-radius-sm)',
                            padding: '8px 12px',
                          }}
                        >
                          <Group>
                            <ActionIcon
                              {...provided.dragHandleProps}
                              variant="subtle"
                            >
                              <IconGripVertical size={18} />
                            </ActionIcon>
                            <Text style={{ flex: 1 }}>{game.name}</Text>

                            <ActionIcon 
                              color="red" 
                              variant="subtle" 
                              onClick={() => deleteGame(game.position)}
                            >
                              <IconTrash size={18} />
                            </ActionIcon>
                          </Group>
                        </List.Item>
                      )}
                    </Draggable>
                  ))}
                {provided.placeholder}
              </List>
            )}
          </Droppable>
        </DragDropContext>
      </Card>

      <Card withBorder shadow="sm" p="lg">
        <Title order={3} mb="md">Proxy</Title>
        
        <Group mb="md">
          <TextInput
            placeholder="Proxy URL"
            value={newProxy}
            onChange={(e) => setNewProxy(e.currentTarget.value)}
            style={{ flex: 1 }}
          />
          <Button onClick={addProxy}>Add</Button>
        </Group>

        <Stack gap="xs">
          {proxies.map((proxy) => (
            <Group key={proxy.url} justify="space-between" p="xs" style={{ border: '1px solid var(--mantine-color-gray-3)', borderRadius: 'var(--mantine-radius-sm)' }}>
              <Text>{proxy.url}</Text>
              <ActionIcon color="red" variant="subtle" onClick={() => deleteProxy(proxy.url)}>
                <IconTrash size={18} />
              </ActionIcon>
            </Group>
          ))}
        </Stack>
      </Card>

      <Card withBorder shadow="sm" p="lg">
        <Group justify="space-between">
          <Title order={3}>Autostart</Title>
          <Switch
            checked={autostart}
            onChange={(e) => setAutostart(e.currentTarget.checked)}
            label="Enable autostart"
            labelPosition="left"
          />
        </Group>
      </Card>
    </Stack>
  );
}