import './App.css'
import {ChatForm} from './components/ChatForm';

import '@radix-ui/themes/styles.css';
import { Theme } from '@radix-ui/themes';


function App() {
  return (
    <Theme>
      {/* <Box>
        <Link href="https://vitejs.dev" target="_blank" rel="noreferrer">
          <img src={viteLogo} className="logo" alt="Vite logo" />
        </Link>
        <Link href="https://react.dev" target="_blank" rel="noreferrer">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </Link>
      </Box>
      <Heading>Vite + React</Heading>
      <Card>
        <Flex align="center" direction="column">
          <Button onClick={() => { setCount((count) => count + 1); }}>
            count is {count}
          </Button>
          <Text as="p">
            Edit <Code>src/App.tsx</Code> and save to test HMR
          </Text>
        </Flex>
      </Card>
      <Text as="p">
        Click on the Vite and React logos to learn more
      </Text> */}
      <ChatForm onSubmit={console.log}/>
    </Theme>
  )
}

export default App
