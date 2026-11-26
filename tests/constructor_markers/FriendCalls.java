package markers.consumer;
public final class FriendCalls {
    public static void main(String[] args) {
        if (InternalKt.friendValue() != 19L) throw new AssertionError("friend constructor default");
        System.out.println("constructor-friend-ok");
    }
}
