package probe

import "syscall"

func Who() uintptr {
	r, _, _ := syscall.Syscall(110, 0, 0, 0)
	return r
}
