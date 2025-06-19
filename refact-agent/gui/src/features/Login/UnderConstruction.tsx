import {
  Box,
  Container,
  Flex,
  Heading,
  Separator,
  Text,
} from "@radix-ui/themes";
import { motion } from "framer-motion";
import { RefactIcon } from "../Providers/icons/Refact";
import { RocketIcon, UpdateIcon } from "@radix-ui/react-icons";

export const UnderConstruction = () => {
  return (
    <Container>
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6 }}
      >
        <Flex direction="column" align="center" justify="center" gap="6" py="9">
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            transition={{
              duration: 0.5,
            }}
          >
            <motion.div
              animate={{ scale: [1, 1.07, 1] }}
              transition={{
                repeat: Infinity,
                duration: 1.2,
                repeatDelay: 2,
                stiffness: 200,
                delay: 2,
              }}
            >
              <RefactIcon width={48} height={48} />
            </motion.div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6, delay: 0.4 }}
          >
            <Heading align="center" as="h1" size="8" mb="2">
              Under Construction
            </Heading>
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6, delay: 0.6 }}
          >
            <Heading
              align="center"
              as="h2"
              size="4"
              color="gray"
              weight="regular"
            >
              Login System in Development
            </Heading>
          </motion.div>
          <motion.div
            initial={{ opacity: 0, scale: 0.9, y: 30 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            transition={{
              duration: 0.7,
              delay: 0.8,
              type: "spring",
              stiffness: 100,
            }}
            style={{ maxWidth: "500px" }}
          >
            <Box
              p="6"
              style={{
                background: "var(--color-surface)",
                border: "1px solid var(--gray-6)",
                borderRadius: "var(--radius-4)",
              }}
            >
              <Flex direction="column" gap="4" align="center">
                <motion.div
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ duration: 0.5, delay: 1.0 }}
                >
                  <Flex align="center" gap="2">
                    <RocketIcon
                      width="20"
                      height="20"
                      color="var(--accent-9)"
                    />
                    <Text size="4" weight="bold">
                      Pre-Release Version
                    </Text>
                  </Flex>
                </motion.div>

                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ duration: 0.6, delay: 1.2 }}
                >
                  <Text size="3" align="center" color="gray">
                    You&apos;re using an early access version of Refact.ai! Our
                    authentication system is currently being enhanced with new
                    features and improved security.
                  </Text>
                </motion.div>

                <motion.div
                  initial={{ scaleX: 0 }}
                  animate={{ scaleX: 1 }}
                  transition={{ duration: 0.5, delay: 1.4 }}
                  style={{ width: "100%" }}
                >
                  <Separator size="4" style={{ width: "100%" }} />
                </motion.div>

                <motion.div
                  initial={{ opacity: 0, y: 15 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.6, delay: 1.6 }}
                >
                  <Flex direction="column" gap="3" align="center">
                    <Text size="2" weight="bold">
                      🔧 What we&apos;re working on:
                    </Text>
                    <Box>
                      <Text size="2" as="div" style={{ lineHeight: "1.6" }}>
                        <Flex direction="column" gap="1" align="start">
                          {[
                            "• Enhanced OAuth integration",
                            "• Seamless IDE authentication",
                          ].map((item, index) => (
                            <motion.div
                              key={index}
                              initial={{ opacity: 0, x: -10 }}
                              animate={{ opacity: 1, x: 0 }}
                              transition={{
                                duration: 0.4,
                                delay: 1.8 + index * 0.1,
                              }}
                            >
                              <Text>{item}</Text>
                            </motion.div>
                          ))}
                        </Flex>
                      </Text>
                    </Box>
                  </Flex>
                </motion.div>

                <motion.div
                  initial={{ scaleX: 0 }}
                  animate={{ scaleX: 1 }}
                  transition={{ duration: 0.5, delay: 2.2 }}
                  style={{ width: "100%" }}
                >
                  <Separator size="4" style={{ width: "100%" }} />
                </motion.div>

                <motion.div
                  initial={{ opacity: 0, y: 15 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.6, delay: 2.4 }}
                >
                  <Flex direction="column" gap="2" align="center">
                    <Flex align="center" gap="2">
                      <motion.div
                        animate={{
                          rotate: 360,
                        }}
                        transition={{
                          duration: 2,
                          repeat: Infinity,
                          ease: "linear",
                        }}
                        style={{
                          display: "flex",
                          justifyContent: "center",
                          alignItems: "center",
                        }}
                      >
                        <UpdateIcon
                          width="16"
                          height="16"
                          color="var(--green-9)"
                        />
                      </motion.div>
                      <Text size="2" weight="bold">
                        Coming Soon
                      </Text>
                    </Flex>
                    <Text size="2" align="center" color="gray">
                      Login functionality will be available in the next release.
                      Thank you for your patience!
                    </Text>
                  </Flex>
                </motion.div>
              </Flex>
            </Box>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6, delay: 2.6 }}
            style={{ maxWidth: "400px" }}
          >
            <Text size="2" align="center" color="gray">
              💡 <strong>Tip:</strong> Keep an eye on our updates for the latest
              features and improvements. This pre-release version gives you
              early access to cutting-edge AI coding tools!
            </Text>
          </motion.div>
        </Flex>
      </motion.div>
    </Container>
  );
};
