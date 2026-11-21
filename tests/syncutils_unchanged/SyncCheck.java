import java.io.IOException;
import java.util.List;
import yuku.alkitab.base.sync.Sync.Entity;
import yuku.alkitab.base.sync.SyncUtils;

public class SyncCheck {
    public static void main(String[] args) throws Exception {
        Entity<String> first = new Entity<>("kind", "gid", new String("content"));
        Entity<String> equal = new Entity<>("kind", "gid", new String("content"));
        Entity<String> wrongId = new Entity<>("kind", "other", "content");
        Entity<String> wrongKind = new Entity<>("other", "gid", "content");
        Entity<String> different = new Entity<>("kind", "gid", null);
        if (!SyncUtils.isSameContent(first, equal) || SyncUtils.isSameContent(first, wrongId)
                || SyncUtils.isSameContent(first, wrongKind) || SyncUtils.isSameContent(first, different)
                || !SyncUtils.isSameContent(different, different)) throw new AssertionError();
        if (SyncUtils.findEntity(List.of(wrongKind, wrongId, first, equal), "gid", "kind") != first
                || SyncUtils.findEntity(List.of(), "gid", "kind") != null) throw new AssertionError();
        int[] calls = {0};
        SyncUtils.wontThrow(() -> calls[0]++);
        if (calls[0] != 1) throw new AssertionError();
        IOException expected = new IOException("failure");
        SyncUtils.ThrowEverythingRunnable task = new SyncUtils.ThrowEverythingRunnable() {
            public void run() throws Exception { throw expected; }
            public String toString() { return "task"; }
        };
        try {
            SyncUtils.wontThrow(task);
            throw new AssertionError();
        } catch (RuntimeException wrapped) {
            if (wrapped.getCause() != expected || !wrapped.getMessage().endsWith(": task")) throw new AssertionError(wrapped);
        }
        if (!List.of(SyncUtils.ThrowEverythingRunnable.class.getMethod("run").getExceptionTypes())
                .equals(List.of(Exception.class))) throw new AssertionError();
    }
}
