<script setup>
const metrics = [
  { value: '3000+', label: '技能培训覆盖人次' },
  { value: '85%', label: '目标就业转化率' },
  { value: '50+', label: '合作企业与岗位资源' },
  { value: 'AI+', label: '未来技能增长入口' }
]

const advantages = [
  {
    title: '退役军人专属成长路径',
    desc: '围绕转岗、再就业、再提升三类核心需求，提供更清晰的分层训练与就业支持。'
  },
  {
    title: '就业导向的课程设计',
    desc: '把能力训练、岗位需求、实战项目和就业推荐串成一个闭环，不只教知识，也交付结果。'
  },
  {
    title: '面向未来的 AI 入口',
    desc: '为后续 AI 培训、AIGC 应用、智能办公和数字化转型预留清晰入口，方便业务持续扩展。'
  },
  {
    title: '可信的服务品牌',
    desc: '用庄重、专业、现代的视觉语言建立信任，让官网既有使命感，也有科技企业的气质。'
  }
]

const programs = [
  {
    tag: '基础能力',
    title: '职业转型训练营',
    desc: '简历优化、面试表达、办公基础、岗位认知、职业规划，帮助退役军人快速适应职场。'
  },
  {
    tag: '实用技能',
    title: '数字办公与运营实训',
    desc: '表格处理、文档协同、演示表达、短视频运营、直播电商基础等，强化岗位可用性。'
  },
  {
    tag: 'AI 进阶',
    title: 'AI 工具应用课程',
    desc: 'AIGC 内容生成、智能写作、智能设计、自动化办公与 AI 辅助决策，面向未来工作方式。'
  }
]

const aiTracks = [
  'AI 办公提效',
  'AIGC 内容生成',
  '智能客服与运营',
  '数据分析辅助',
  '数字化管理工具',
  'AI 职业适配训练'
]

const process = [
  { step: '01', title: '需求评估', desc: '了解个人背景、技能基础和就业意向，形成适配方案。' },
  { step: '02', title: '分层培训', desc: '按基础、进阶、AI 三条路径组织课程，确保学得会、用得上。' },
  { step: '03', title: '岗位衔接', desc: '通过企业合作和岗位推荐，推进从培训到就业的转换。' },
  { step: '04', title: '持续跟踪', desc: '提供就业回访、能力补强和进阶学习入口，形成长期陪伴。' }
]

const outcomes = [
  { title: '培训人数', value: '3200+' },
  { title: '就业推荐', value: '1800+' },
  { title: '合作单位', value: '60+' },
  { title: '课程模块', value: '24' }
]

const contacts = [
  { label: '服务对象', value: '退役军人 / 家属 / 合作企业' },
  { label: '业务方向', value: '职业培训 / 就业支持 / AI 业务拓展' },
  { label: '品牌名称', value: '上海迷彩侠实业有限公司' }
]
</script>

<template>
  <div id="top" class="page-shell">
    <div class="ambient ambient-left"></div>
    <div class="ambient ambient-right"></div>
    <div class="grid-overlay"></div>

    <header class="topbar">
      <div class="brand">
        <div class="brand-mark">MX</div>
        <div>
          <p class="brand-name">上海迷彩侠实业有限公司</p>
          <p class="brand-subtitle">退役军人技能培训与就业支持</p>
        </div>
      </div>

      <nav class="nav">
        <a href="#about">品牌定位</a>
        <a href="#programs">培训体系</a>
        <a href="#ai">AI 入口</a>
        <a href="#results">成果数据</a>
        <a href="#contact">联系合作</a>
      </nav>
    </header>

    <main>
      <section class="hero">
        <div class="hero-copy">
          <p class="eyebrow">Veteran Career Platform · AI Enabled Future</p>
          <h1>退役不褪色，AI 赋能再出发</h1>
          <p class="hero-text">
            让退役军人的荣誉感、执行力和学习力，转化为新的职业竞争力。
            我们以培训、就业与科技能力为核心，打造面向未来的成长门户。
          </p>

          <div class="hero-actions">
            <a class="btn btn-primary" href="#programs">了解培训体系</a>
            <a class="btn btn-secondary" href="#contact">查看合作方式</a>
          </div>

          <div class="hero-points">
            <span>可信</span>
            <span>专业</span>
            <span>有温度</span>
            <span>面向未来</span>
          </div>
        </div>

        <div class="hero-panel">
          <div class="hero-card hero-card-main">
            <p class="card-label">官网定位</p>
            <h2>从军营到职场的现代化转型入口</h2>
            <p>
              用更现代的 UI、更清晰的服务路径和更明确的成果展示，让用户一眼看见
              “这是一个认真做退役军人就业服务的品牌”。
            </p>
            <div class="card-mini-stats">
              <div>
                <strong>04</strong>
                <span>核心服务模块</span>
              </div>
              <div>
                <strong>24</strong>
                <span>课程与能力单元</span>
              </div>
            </div>
          </div>

          <div class="hero-card hero-card-side">
            <p class="card-label">AI 业务预留</p>
            <ul class="ai-list">
              <li v-for="item in aiTracks" :key="item">{{ item }}</li>
            </ul>
          </div>
        </div>
      </section>

      <section class="metric-strip" aria-label="关键数据">
        <article v-for="metric in metrics" :key="metric.label" class="metric-card">
          <strong>{{ metric.value }}</strong>
          <span>{{ metric.label }}</span>
        </article>
      </section>

      <section id="about" class="section">
        <div class="section-head">
          <p class="eyebrow">Brand Positioning</p>
          <h2>官网应该传递的氛围</h2>
          <p>
            庄重但不沉闷，专业但不冰冷，有军人体系的秩序感，也有科技企业的未来感。
          </p>
        </div>

        <div class="grid-two">
          <article class="content-card statement-card">
            <h3>我们希望用户感受到什么</h3>
            <ul class="check-list">
              <li>这是一家可信的正规公司，不是包装型培训机构。</li>
              <li>这里关注的是退役军人的长期职业发展，不只是一次培训。</li>
              <li>AI 不是噱头，而是未来业务和岗位能力的重要延展。</li>
              <li>页面要有力量感，也要有温度和陪伴感。</li>
            </ul>
          </article>

          <article class="content-card theme-card">
            <h3>推荐主题关键词</h3>
            <div class="chip-wrap">
              <span v-for="item in ['可信', '稳重', '未来感', '使命感', '成长', '就业', '科技']" :key="item" class="chip">
                {{ item }}
              </span>
            </div>
            <p class="muted-copy">
              可以把整站理解成“退役军人职业发展平台”，而不是单一课程宣传页。
            </p>
          </article>
        </div>
      </section>

      <section class="section">
        <div class="section-head">
          <p class="eyebrow">Why Us</p>
          <h2>为什么这个品牌值得信任</h2>
          <p>把退役军人的荣誉感、就业结果和 AI 未来入口一起讲清楚，形成更完整的品牌认知。</p>
        </div>

        <div class="cards-grid">
          <article v-for="item in advantages" :key="item.title" class="content-card program-card">
            <h3>{{ item.title }}</h3>
            <p>{{ item.desc }}</p>
          </article>
        </div>
      </section>

      <section id="programs" class="section">
        <div class="section-head">
          <p class="eyebrow">Training System</p>
          <h2>培训体系与业务结构</h2>
          <p>用模块化结构展示“基础能力 + 实用技能 + AI 进阶”，让业务逻辑一目了然。</p>
        </div>

        <div class="cards-grid">
          <article v-for="program in programs" :key="program.title" class="content-card program-card">
            <p class="card-label">{{ program.tag }}</p>
            <h3>{{ program.title }}</h3>
            <p>{{ program.desc }}</p>
          </article>
        </div>
      </section>

      <section id="ai" class="section split-section">
        <div class="section-head narrow">
          <p class="eyebrow">AI Gateway</p>
          <h2>把 AI 做成官网里的正式入口</h2>
          <p>
            不是把 AI 做得花哨，而是让它成为一个清晰、可扩展、可持续迭代的业务模块。
          </p>
        </div>

        <div class="content-card ai-card">
          <div class="ai-visual">
            <div class="ai-orbit"></div>
            <div class="ai-core">AI</div>
          </div>

          <div class="ai-copy">
            <h3>未来培训方向建议</h3>
            <p>
              AI 办公提效、AIGC 内容生成、智能运营、数据分析辅助、自动化工具应用，
              这些方向都适合和退役军人的纪律性、执行力结合，形成新的就业竞争力。
            </p>

            <div class="ai-tags">
              <span v-for="item in aiTracks" :key="item">{{ item }}</span>
            </div>
          </div>
        </div>
      </section>

      <section class="section">
        <div class="section-head">
          <p class="eyebrow">Service Flow</p>
          <h2>服务闭环：从报名到就业跟踪</h2>
          <p>让用户看到一条清晰路径，减少“不知道下一步做什么”的犹豫。</p>
        </div>

        <div class="timeline">
          <article v-for="item in process" :key="item.step" class="timeline-item">
            <span class="timeline-step">{{ item.step }}</span>
            <div>
              <h3>{{ item.title }}</h3>
              <p>{{ item.desc }}</p>
            </div>
          </article>
        </div>
      </section>

      <section id="results" class="section">
        <div class="section-head">
          <p class="eyebrow">Impact</p>
          <h2>成果展示建议</h2>
          <p>数据要有，但呈现方式要克制，像“可信的品牌概览”，而不是报表页面。</p>
        </div>

        <div class="results-grid">
          <article v-for="item in outcomes" :key="item.title" class="result-card">
            <strong>{{ item.value }}</strong>
            <span>{{ item.title }}</span>
          </article>
        </div>
      </section>

      <section id="contact" class="section contact-section">
        <div class="section-head narrow">
          <p class="eyebrow">Contact</p>
          <h2>联系与合作入口</h2>
          <p>适合放企业合作、培训报名、AI 业务咨询和就业资源对接。</p>
        </div>

        <div class="contact-card content-card">
          <div v-for="item in contacts" :key="item.label" class="contact-row">
            <span>{{ item.label }}</span>
            <strong>{{ item.value }}</strong>
          </div>
          <div class="contact-actions">
            <a class="btn btn-primary" href="mailto:hello@micaixia.com">发送合作邮件</a>
            <a class="btn btn-secondary" href="#top">回到顶部</a>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>
