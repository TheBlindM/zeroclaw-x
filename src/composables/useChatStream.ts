import { onBeforeUnmount, onMounted } from "vue";
import {
  onChatApprovalRequest,
  onChatContext,
  onChatDone,
  onChatError,
  onChatToken
} from "@/api/tauri";
import { useChatStore } from "@/stores/chat";

export function useChatStream() {
  const chatStore = useChatStore();
  const disposers: Array<() => void> = [];

  onMounted(async () => {
    disposers.push(
      await onChatApprovalRequest((payload) => {
        chatStore.addApprovalRequest(payload);
      })
    );

    disposers.push(
      await onChatContext((payload) => {
        chatStore.setContextPreview(payload);
      })
    );

    disposers.push(
      await onChatToken(({ session_id: sessionId, token }) => {
        chatStore.appendAssistantDelta(sessionId, token);
      })
    );

    disposers.push(
      await onChatDone(({ session_id: sessionId }) => {
        chatStore.finishAssistantMessage(sessionId);
      })
    );

    disposers.push(
      await onChatError(({ session_id: sessionId, error }) => {
        chatStore.markAssistantError(sessionId, error);
      })
    );
  });

  onBeforeUnmount(() => {
    for (const dispose of disposers.splice(0, disposers.length)) {
      dispose();
    }
  });
}