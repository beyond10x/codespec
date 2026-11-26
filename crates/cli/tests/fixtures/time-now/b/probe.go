package probe

import "time"

func Stamp() int64 {
	return time.Now().Unix()
}
