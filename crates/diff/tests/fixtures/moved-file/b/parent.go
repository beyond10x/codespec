package probe

import "syscall"

func Parent() uintptr {
	r, _, _ := syscall.Syscall(110, 0, 0, 0)
	return r
}
