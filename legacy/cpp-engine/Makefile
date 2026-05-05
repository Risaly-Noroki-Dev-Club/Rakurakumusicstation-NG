CXX      := g++
CXXFLAGS := -std=c++17 -O2 -g -Wall -Wextra -I. -Isrc
LDFLAGS  := -lpthread -lssl -lcrypto

SRCS := src/main.cpp \
        src/radio_server.cpp \
        src/audio_player.cpp \
        src/web_server.cpp \
        src/stream_server.cpp \
        src/client_connection.cpp \
        src/metadata.cpp

OBJS := $(SRCS:.cpp=.o)
DEPS := $(OBJS:.o=.d)
TARGET := radioserver

.PHONY: all clean debug test

all: $(TARGET)

$(TARGET): $(OBJS)
	$(CXX) $(CXXFLAGS) -o $@ $(OBJS) $(LDFLAGS)

-include $(DEPS)

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -MMD -MP -c $< -o $@

debug: CXXFLAGS := -std=c++17 -g -O0 -Wall -Wextra -I. -Isrc
debug: clean $(TARGET)

clean:
	rm -f $(OBJS) $(DEPS) $(TARGET)

test:
	@echo "Tests not yet implemented."
