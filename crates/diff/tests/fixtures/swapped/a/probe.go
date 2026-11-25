package probe

import "syscall"

func Ids() (uintptr, uintptr) {
	pid, _, _ := syscall.Syscall(39, 0, 0, 0)
	ppid, _, _ := syscall.RawSyscall(110, 0, 0, 0)
	return pid, ppid
}
