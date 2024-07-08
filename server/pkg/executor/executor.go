package executor

import (
	"github.com/ethos-works/InfinityVM/server/pkg/db"
	"github.com/ethos-works/InfinityVM/server/pkg/queue"
	"github.com/ethos-works/InfinityVM/server/pkg/types"
)

type Executor struct {
	db    db.DB
	queue queue.Queue[types.Job]
}
