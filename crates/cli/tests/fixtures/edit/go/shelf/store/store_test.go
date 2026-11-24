package store

import "testing"

func TestSpare(t *testing.T) {
	if spare() != 5 {
		t.Fatal("spare is not 5")
	}
}
