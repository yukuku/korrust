import java.util.*;
import yuku.alkitab.base.audio.*;
import compatibility.*;

public final class PlaybackCheck {
    static int checks;
    static void eq(Object value, Object expected) {
        checks++;
        if (!Objects.equals(value, expected)) throw new AssertionError(value + " != " + expected);
    }
    public static void main(String[] args) {
        PlaybackState idle = PlaybackState.Companion.getIDLE();
        eq(idle.isPlaying(), false); eq(idle.getPreparing(), false);
        eq(idle.getVersionId(), ""); eq(idle.getAudioId(), "");
        eq(idle.getBookId(), -1); eq(idle.getChapter_1(), 0); eq(idle.getVerse_1(), 0);
        eq(idle.getPositionMs(), 0L); eq(idle.getDurationMs(), 0L);
        eq(idle.getSpeed(), 1.0f); eq(idle.getError(), null); eq(idle.getLogs(), List.of());
        eq(idle.isActive(), false); eq(PlaybackState.Companion.getIDLE() == idle, true);
        AudioLogEntry log = new AudioLogEntry(17L, "loaded");
        PlaybackState active = idle.copy(true, false, "v", "a", 3, 2, 1, 7L, 11L, 1.5f, "error", List.of(log));
        eq(active.isActive(), true); eq(active.component5(), 3); eq(active.component12(), List.of(log));
        eq(active.getPositionMs(), 7L); eq(active.getDurationMs(), 11L); eq(active.getError(), "error");
        eq(active.equals(idle), false);
        eq(idle.equals(idle.copy(false, false, "", "", -1, 0, 0, 0L, 0L, 1.0f, null, List.of())), true);
        eq(idle.hashCode(), idle.copy(false, false, "", "", -1, 0, 0, 0L, 0L, 1.0f, null, List.of()).hashCode());
        eq(Snapshot.Companion.getEMPTY().getLogs(), List.of());
        eq(ExpectedResultKt.nullableList(), List.of());
        boolean rejected = false;
        try { new PlaybackState(false, false, null, "", -1, 0, 0, 0L, 0L, 1.0f, null, List.of()); }
        catch (NullPointerException expected) { rejected = true; }
        eq(rejected, true);
        System.out.println("checks=" + checks);
    }
}
