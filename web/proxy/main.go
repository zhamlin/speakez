package main

import (
	"bufio"
	"context"
	"encoding/binary"
	"flag"
	"fmt"
	"io"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/signal"
	"speakez-proxy/mumble"
	"time"

	"github.com/coder/websocket"
	"github.com/zhamlin/httpwatch"
)

type config struct {
	httpwatch.WatcherConfig
	addr       string
	tlsCert    string
	tlsKey     string
	socketPath string
}

func (c config) hasTLS() bool {
	return c.tlsKey != "" && c.tlsCert != ""
}

func loadConfig() config {
	cfg := config{}

	flag.StringVar(&cfg.addr, "addr", ":8080", "address to listen on")
	flag.StringVar(&cfg.tlsCert, "cert", "", "tls cert")
	flag.StringVar(&cfg.tlsKey, "key", "", "tls key")
	flag.StringVar(&cfg.socketPath, "socket", "/tmp/speakez.sock", "unix socket for speakez")

	flag.StringVar(&cfg.Dir, "dir", "", "directory to serve via /")
	flag.StringVar(&cfg.FilePattern, "pattern", "", "file matching pattern")
	flag.BoolVar(&cfg.Recursive, "recursive", true, "watch all files recursively")

	flag.Parse()
	return cfg
}

func newWebsocketHandler(socketPath string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		c, err := websocket.Accept(w, r, &websocket.AcceptOptions{
			InsecureSkipVerify: true,
		})
		if err != nil {
			slog.Error("websocket.Accept", "err", err)
			return
			// ...
		}
		defer func() {
			c.CloseNow()
			slog.Info("websocket closed")
		}()

		slog.Info("websocket connected")

		speakezConn, err := net.Dial("unix", socketPath)
		if err != nil {
			slog.Error("net.Dial", "err", err)
			return
		}
		defer speakezConn.Close()

		ctx := r.Context()
		done := make(chan struct{})
		maybeClose := func() {
			select {
			case <-done:
			default:
				close(done)
			}
		}

		// weboscket -> unix socket
		go func() {
			defer maybeClose()
			buf := make([]byte, 4096)

			for {
				typ, reader, err := c.Reader(ctx)
				if err != nil {
					slog.Error("c.Reader", "err", err)
					return
				}

				switch typ {
				case websocket.MessageBinary:
					// mumbleType := binary.BigEndian.Uint16(data[2:])
					// slog.Info("message received", "type", typ, "size", len(data), "mumbleType", mumbleType)
					// lw := iox.NewLimitedWriter(2)
					// reader = io.TeeReader(reader, lw)
					_, err := io.CopyBuffer(speakezConn, reader, buf)
					if err != nil {
						slog.Error("io.Copy", "err", err, "type", typ, "from", "websocket", "to", "unix_socket")
						return
					}
				case websocket.MessageText:
					// n, err := io.CopyBuffer(buf)
					// slog.Info("message received", "type", typ, "msg", string(data))
				default:
					err := fmt.Errorf("unexpected message type: %v", typ)
					slog.Error("c.Read", "err", err)
				}
			}
		}()

		// unix socket -> weboscket
		go func() {
			defer maybeClose()

			messageBuffer := make([]byte, 4096*2)
			reader := bufio.NewReaderSize(speakezConn, 4096)

			for {
				headerBuf := messageBuffer[:mumble.PREFIX_HEADER_SIZE]
				_, err := io.ReadFull(reader, headerBuf)
				if err != nil {
					slog.Error("failed to read message header: io.ReadFull", "err", err)
					return
				}

				// typ := headerBuf[:mumble.PREFIX_TYPE_BYTES]
				// mumbleType := binary.BigEndian.Uint16(typ)

				size := headerBuf[mumble.PREFIX_TYPE_BYTES:]
				messageSize := binary.BigEndian.Uint32(size)
				totalMessageSize := mumble.PREFIX_HEADER_SIZE + messageSize

				if l := len(messageBuffer); l < int(totalMessageSize) {
					slog.Error("message size was greater than the message buf", "len(messageBuf)", l, "message_size", totalMessageSize)
					return
				}

				_, err = io.ReadFull(reader, messageBuffer[mumble.PREFIX_HEADER_SIZE:totalMessageSize])
				if err != nil {
					slog.Error("failed to read message body: io.ReadFull", "err", err)
					return
				}

				// slog.Info("speakezConn sent data to websocket", "type", mumbleType, "size", len(messageBuffer), "mumble message size", messageSize)
				err = c.Write(ctx, websocket.MessageBinary, messageBuffer[:totalMessageSize])
				if err != nil {
					slog.Error("websocket.Write", "err", err)
					return
				}
			}
		}()

		<-done

		slog.Info("web socket closing normally")
		c.Close(websocket.StatusNormalClosure, "")
	}
}

func main() {
	cfg := loadConfig()

	if err := run(cfg); err != nil {
		fmt.Fprintf(os.Stderr, "%s\n", err)
		os.Exit(1)
	}
}

func run(cfg config) (err error) {
	lh := slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{AddSource: false})
	slog.SetDefault(slog.New(lh))
	// Handle SIGINT (CTRL+C) gracefully.
	ctx, signalCancle := signal.NotifyContext(context.Background(), os.Interrupt)
	defer signalCancle()

	h := http.NewServeMux()
	h.Handle("GET /ws", newWebsocketHandler(cfg.socketPath))

	if cfg.FilePattern != "" {
		b := httpwatch.NewBroadcaster()
		fn, err := httpwatch.NewWatcherFn(ctx, cfg.WatcherConfig, b)
		if err != nil {
			return fmt.Errorf("createWatcherFn: %w", err)
		}
		go fn()
		h.Handle("GET /_/events", httpwatch.NewWebsocketHandler(b))
	}

	if cfg.Dir != "" {
		headers := http.Header{}
		headers.Set("Cross-Origin-Opener-Policy", "same-origin")
		headers.Set("Cross-Origin-Embedder-Policy", "require-corp")
		headers.Set("Cache-Control", "max-age=0")
		headers.Set("Access-Control-Allow-Origin", "*")

		headersMW := httpwatch.HeaderMiddleware(headers)
		handler := httpwatch.NewFileServer(cfg.Dir, true)
		h.Handle("GET /", headersMW(handler))
	}

	s := http.Server{
		Addr:    cfg.addr,
		Handler: h,
		BaseContext: func(_ net.Listener) context.Context {
			return ctx
		},
		ReadTimeout:  time.Second,
		WriteTimeout: 10 * time.Second,
		IdleTimeout:  20 * time.Second,
	}

	srvErr := make(chan error, 1)
	go func() {
		slog.Info("server listening", "addr", cfg.addr)

		listenAndServe := s.ListenAndServe
		if cfg.hasTLS() {
			listenAndServe = func() error {
				return s.ListenAndServeTLS(cfg.tlsCert, cfg.tlsKey)
			}
		}

		if err := listenAndServe(); err != http.ErrServerClosed {
			srvErr <- err
		}
	}()

	select {
	case err = <-srvErr:
		return err
	case <-ctx.Done():
	}

	slog.Info("shutting down")
	shutdownCtx, cancel := context.WithTimeout(
		context.Background(), 5*time.Second,
	)
	defer cancel()
	return s.Shutdown(shutdownCtx)
}
