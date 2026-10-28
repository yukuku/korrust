public final class FriendCalls {
 private static int checks;
 private static void require(boolean value) { checks++; if (!value) throw new AssertionError("friend check=" + checks); }
 public static void main(String[] args) {
  visibilityprobe.InternalDefault value = new visibilityprobe.InternalDefault();
  require(visibilityfriend.CandidateFriendKt.touch(value, 101) == 101);
  require(value.getValue$visibility_feature() == 101);
  visibilityprobe.PublicInternalSetter mixed = new visibilityprobe.PublicInternalSetter();
  require(visibilityfriend.CandidateFriendKt.partial(mixed, 103) == 103);
  require(mixed.getValue() == 103);
  require(visibilityfriend.CandidateFriendKt.objectValue(107) == 107);
  require(visibilityprobe.State.INSTANCE.getHidden$visibility_feature() == 107);
  require(visibilityfriend.CandidateFriendKt.classCompanion(109) == 109);
  require(visibilityprobe.Host.Companion.getHidden$visibility_feature() == 109);
  require(visibilityfriend.CandidateFriendKt.interfaceCompanion(113) == 113);
  require(visibilityprobe.Contract.Companion.getHidden$visibility_feature() == 113);
  require(visibilityfriend.CandidateFriendKt.topValue() == 1);
  System.out.println("friend-checks=" + checks);
 }
}
