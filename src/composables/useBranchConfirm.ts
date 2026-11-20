import { ref } from "vue";

export interface BranchConfirmGroup {
  /** Repository name, or empty for a single repo where a heading adds nothing. */
  name: string;
  branches: string[];
}

export interface BranchConfirmOptions {
  title: string;
  message: string;
  groups: BranchConfirmGroup[];
  note?: string;
  confirmLabel: string;
  cancelLabel?: string;
}

interface BranchConfirmRequest extends BranchConfirmOptions {
  resolve: (ok: boolean) => void;
}

const request = ref<BranchConfirmRequest | null>(null);

/**
 * Styled in-app confirmation that lists branches grouped by repository, in
 * place of the native confirm dialog's wall of text.
 */
export function useBranchConfirm() {
  function confirmBranches(options: BranchConfirmOptions): Promise<boolean> {
    request.value?.resolve(false);
    return new Promise((resolve) => {
      request.value = { ...options, resolve };
    });
  }

  function answer(ok: boolean) {
    const current = request.value;
    request.value = null;
    current?.resolve(ok);
  }

  return { request, confirmBranches, answer };
}
