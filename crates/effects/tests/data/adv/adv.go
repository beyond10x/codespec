package adv

import (
	"bufio"
	"io/fs"
	"log"
	"log/slog"
	"path/filepath"
	"slices"
	"sort"
	"sync"
	"sync/atomic"
	"time"
)

func stamp() {
	_ = time.Now()
}

func OnceStamp(o *sync.Once) {
	o.Do(stamp)
}

func before(a, b int) bool {
	return time.Now().Unix() > int64(a+b)
}

func SortBy(x []int) {
	sort.Slice(x, before)
}

func visit(path string, d fs.DirEntry, err error) error {
	log.Print(path)
	return err
}

func Walk(root string) error {
	return filepath.WalkDir(root, visit)
}

func LogLine() {
	log.Print("x")
}

func SlogLine(l *slog.Logger) {
	l.Info("x")
}

func SortInts(x []int) {
	slices.Sort(x)
}

func LoadPtr(p *atomic.Pointer[int]) *int {
	return p.Load()
}

func Flush(rw *bufio.ReadWriter) error {
	return rw.Flush()
}

func Stamp(t time.Time) string {
	return t.Format(time.RFC3339)
}
