import java.util.ArrayList;
import java.util.List;
import yuku.alkitab.songs.MediaController.State;
import yuku.alkitab.songs.MediaStateListener;

public class MediaCheck {
    public static void main(String[] args) {
        List<State> observed = new ArrayList<>();
        MediaStateListener listener = observed::add;
        for (State state : State.values()) listener.onControllerStateChanged(state);
        if (!observed.equals(List.of(State.values()))) throw new AssertionError();
    }
}
