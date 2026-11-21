import yuku.alkitab.base.audio.PlaybackState;
import yuku.alkitab.base.audio.AudioLogEntry;
import java.util.Collections;
public final class Calls {
    public static void main(String[] args) {
        PlaybackState idle = PlaybackState.Companion.getIDLE();
        if (idle.isActive() || idle.getBookId() != -1 || idle.getSpeed() != 1.0f) throw new AssertionError("idle");
        PlaybackState state = new PlaybackState(true, true, "v", "a", 2, 3, 4, 5L, 6L, -0.0f, null, Collections.singletonList(new AudioLogEntry(7L, "log")));
        if (!state.isPlaying() || !state.getPreparing() || !state.isActive() || !state.getVersionId().equals("v") || !state.getAudioId().equals("a")) throw new AssertionError("fields");
        if (state.getChapter_1() != 3 || state.getVerse_1() != 4 || state.getPositionMs() != 5 || state.getDurationMs() != 6) throw new AssertionError("positions");
        if (Float.floatToRawIntBits(state.getSpeed()) != Float.floatToRawIntBits(-0.0f) || state.getError() != null || state.getLogs().get(0).getTimestampMs() != 7L) throw new AssertionError("values");
        PlaybackState copied = state.copy(true, true, "v", "a", 2, 3, 4, 5L, 6L, -0.0f, null, state.getLogs());
        if (!copied.equals(state) || copied.hashCode() != state.hashCode()) throw new AssertionError("copy");
        if (!proof.ConsumerKt.checkModels().equals("OK")) throw new AssertionError("Kotlin consumer");
        System.out.println("OK");
    }
}
