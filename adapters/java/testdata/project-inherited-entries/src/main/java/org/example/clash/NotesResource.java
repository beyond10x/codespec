package org.example.clash;

public class NotesResource implements ReadApi, WriteApi { // refused: two interfaces
    @Override
    public String note(String id) {
        return "note";
    }
}
