import { JSX, useEffect, useState } from 'react';
import { Heart, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useAuth } from '@/contexts/AuthContext';
import {
  CommentResponse,
  createComment,
  deleteComment,
  getComments,
  likePost,
  unlikePost,
} from '@/lib/api';

/**
 * Heart button showing the like count; toggles the current user's like.
 *
 * @param {object} props - The component props
 * @param {string} props.postId - Id of the post
 * @param {number} props.initialCount - Like count at load time
 * @param {boolean} props.initialLiked - Whether the current user liked it at load time
 * @returns {JSX.Element} The like button
 */
export const LikeButton = ({
  postId,
  initialCount,
  initialLiked,
}: {
  postId: string;
  initialCount: number;
  initialLiked: boolean;
}): JSX.Element => {
  const { user } = useAuth();
  const [count, setCount] = useState(initialCount);
  const [liked, setLiked] = useState(initialLiked);
  const [busy, setBusy] = useState(false);

  /**
   * Toggles the like state for the current user.
   */
  const toggle = async (): Promise<void> => {
    if (!user || busy) {
      return;
    }
    setBusy(true);
    try {
      const state = liked ? await unlikePost(postId) : await likePost(postId);
      setCount(state.likeCount);
      setLiked(state.likedByMe);
    } catch (error) {
      console.error('Failed to toggle like:', error);
    } finally {
      setBusy(false);
    }
  };

  return (
    <button
      type="button"
      onClick={() => void toggle()}
      disabled={!user || busy}
      title={user ? (liked ? 'Unlike' : 'Like') : 'Log in to like posts'}
      className="flex cursor-pointer items-center gap-2 disabled:cursor-default"
      data-testid="like-button"
    >
      <Heart size={20} fill={liked ? 'currentColor' : 'none'} />
      <span className="text-base" data-testid="like-count">
        {count}
      </span>
    </button>
  );
};

/**
 * Comment list + composer for a post the visitor can access.
 *
 * @param {object} props - The component props
 * @param {string} props.postId - Id of the post
 * @returns {JSX.Element} The comments section
 */
export const CommentsSection = ({ postId }: { postId: string }): JSX.Element => {
  const { user } = useAuth();
  const [comments, setComments] = useState<CommentResponse[]>([]);
  const [draft, setDraft] = useState('');
  const [replyTo, setReplyTo] = useState<CommentResponse | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    /**
     * Loads the comments for the post.
     */
    const load = async (): Promise<void> => {
      try {
        setComments(await getComments(postId));
      } catch (error) {
        console.error('Failed to load comments:', error);
      }
    };
    void load();
  }, [postId]);

  /**
   * Submits the draft comment (optionally as a reply).
   */
  const submit = async (): Promise<void> => {
    const content = draft.trim();
    if (!content || busy) {
      return;
    }
    setBusy(true);
    try {
      const created = await createComment({
        postId,
        content,
        parentId: replyTo?.id ?? null,
      });
      setComments((current) => [...current, created]);
      setDraft('');
      setReplyTo(null);
    } catch (error) {
      console.error('Failed to post comment:', error);
    } finally {
      setBusy(false);
    }
  };

  /**
   * Deletes one comment and removes it from the list.
   *
   * @param {string} commentId - Id of the comment to delete
   */
  const remove = async (commentId: string): Promise<void> => {
    try {
      await deleteComment(commentId);
      setComments((current) => current.filter((comment) => comment.id !== commentId));
    } catch (error) {
      console.error('Failed to delete comment:', error);
    }
  };

  const topLevel = comments.filter((comment) => !comment.parentId);
  /**
   * Returns the replies of a comment, oldest first.
   *
   * @param {string} parentId - Parent comment id
   * @returns {CommentResponse[]} The replies
   */
  const repliesOf = (parentId: string): CommentResponse[] =>
    comments.filter((comment) => comment.parentId === parentId);

  /**
   * Renders one comment row.
   *
   * @param {object} params - Render params
   * @param {CommentResponse} params.comment - The comment
   * @param {boolean} params.isReply - Indent when true
   * @returns {JSX.Element} The row
   */
  const renderComment = ({
    comment,
    isReply,
  }: {
    comment: CommentResponse;
    isReply: boolean;
  }): JSX.Element => (
    <div key={comment.id} className={`flex gap-3 ${isReply ? 'ml-10' : ''}`} data-testid="comment">
      {comment.authorAvatarUrl && (
        <img src={comment.authorAvatarUrl} alt="" className="h-8 w-8 shrink-0 rounded-none" />
      )}
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-bold">{comment.authorName || 'Member'}</span>
          {comment.isCreator && <span className="bg-black px-1 text-xs text-white">Creator</span>}
          {comment.createdAt && (
            <span className="text-xs">{new Date(comment.createdAt).toLocaleDateString()}</span>
          )}
        </div>
        <p className="text-base whitespace-pre-wrap">{comment.content}</p>
        <div className="flex items-center gap-3">
          {user && !isReply && (
            <button
              type="button"
              className="cursor-pointer text-xs underline"
              onClick={() => setReplyTo(comment)}
            >
              Reply
            </button>
          )}
          {comment.canDelete && (
            <button
              type="button"
              className="cursor-pointer text-xs underline"
              onClick={() => void remove(comment.id)}
              title="Delete comment"
            >
              <Trash2 size={12} />
            </button>
          )}
        </div>
      </div>
    </div>
  );

  return (
    <div className="flex flex-col gap-5 p-10" data-testid="comments-section">
      <h2 className="text-xl font-bold">
        Comments{comments.length > 0 ? ` (${comments.length})` : ''}
      </h2>
      {topLevel.map((comment) => (
        <div key={comment.id} className="flex flex-col gap-3">
          {renderComment({ comment, isReply: false })}
          {repliesOf(comment.id).map((reply) => renderComment({ comment: reply, isReply: true }))}
        </div>
      ))}
      {user ? (
        <div className="flex flex-col gap-2">
          {replyTo && (
            <p className="text-sm">
              Replying to {replyTo.authorName || 'Member'}{' '}
              <button
                type="button"
                className="cursor-pointer underline"
                onClick={() => setReplyTo(null)}
              >
                cancel
              </button>
            </p>
          )}
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="Write a comment..."
            maxLength={5000}
            rows={3}
            className="w-full border-3 border-black p-3 text-base"
            data-testid="comment-input"
          />
          <Button
            containerClassName="w-max"
            disabled={busy || !draft.trim()}
            onClick={() => void submit()}
          >
            {busy ? 'Posting...' : 'Post comment'}
          </Button>
        </div>
      ) : (
        <p className="text-sm">Log in to join the conversation.</p>
      )}
    </div>
  );
};
